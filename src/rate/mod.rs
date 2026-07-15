mod agent;
pub mod auth;
mod cache;
mod star;
mod submit;

use async_std::channel::Sender;

use crate::{
	channel::{Action, Status, report},
	entry::{Entry, Source},
};

use star::Star;

pub type Rating = (Source, Star);

pub fn pending(list: &[Entry]) -> hmerr::Result<Vec<Rating>> {
	let submitted = cache::read()?;

	let rating = list
		.iter()
		.map(|entry| star::from_q(entry.q).map(|star| (entry.s.clone(), star)))
		.collect::<hmerr::Result<Vec<Rating>>>()?;

	Ok(rating
		.into_iter()
		.filter(|(source, star)| submitted.get(source) != Some(star))
		.collect())
}

pub async fn sync(bearer: String, pending: Vec<Rating>, tx: Sender<Status>) {
	let mut submitted = match cache::read() {
		Ok(submitted) => submitted,
		Err(e) => {
			report(
				&tx,
				Status {
					action: Action::SubmitRating(pending.len()),
					status: Err(e.to_string()),
				},
			)
			.await;
			return;
		}
	};

	for (index, chunk) in pending.chunks(submit::CHUNK).enumerate() {
		if index > 0 {
			async_std::task::sleep(submit::RATE_LIMIT).await;
		}

		let status = submit::submit(&bearer, chunk)
			.and_then(|()| {
				submitted.extend(chunk.iter().cloned());
				cache::write(&submitted)
			})
			.map_err(|e| e.to_string());

		report(
			&tx,
			Status {
				action: Action::SubmitRating(chunk.len()),
				status,
			},
		)
		.await;
	}
}

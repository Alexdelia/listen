mod agent;
pub mod auth;
mod cache;
mod submit;
mod value;

use async_std::channel::Sender;

use crate::declaration::{Entry, Source};

use super::channel::{Action, Status, report};

use value::Value;

pub type Rating = (Source, Value);

pub struct Pending {
	submitted: cache::Submitted,
	pub rating: Vec<Rating>,
}

pub fn pending(list: &[Entry]) -> hmerr::Result<Pending> {
	let submitted = cache::read()?;

	let rating = list
		.iter()
		.map(|entry| (entry.s.clone(), value::from_q(entry.q)))
		.filter(|(source, value)| submitted.get(source) != Some(value))
		.collect();

	Ok(Pending { submitted, rating })
}

pub async fn sync(bearer: String, pending: Pending, tx: Sender<Status>) {
	let Pending {
		mut submitted,
		rating,
	} = pending;

	for (index, chunk) in rating.chunks(submit::CHUNK).enumerate() {
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

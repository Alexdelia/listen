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
	client: auth::Client,
	submitted: cache::Submitted,
	pub rating: Vec<Rating>,
}

pub fn pending(list: &[Entry]) -> hmerr::Result<Option<Pending>> {
	let Some(client) = auth::client() else {
		return Ok(None);
	};

	let submitted = cache::read()?;

	let rating = list
		.iter()
		.map(|entry| (entry.s, value::from_q(entry.q)))
		.filter(|(source, value)| submitted.get(source) != Some(value))
		.collect();

	Ok(Some(Pending {
		client,
		submitted,
		rating,
	}))
}

pub fn acquire(pending: &Pending) -> hmerr::Result<Option<String>> {
	auth::acquire(&pending.client)
}

pub async fn sync(bearer: String, pending: Pending, tx: Sender<Status>) {
	let Pending {
		mut submitted,
		rating,
		..
	} = pending;

	for (index, chunk) in rating.chunks(submit::CHUNK).enumerate() {
		if index > 0 {
			async_std::task::sleep(submit::RATE_LIMIT).await;
		}

		let status = submit::submit(&bearer, chunk)
			.and_then(|()| {
				submitted.extend(chunk.iter().copied());
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

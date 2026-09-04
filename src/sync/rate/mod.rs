pub(super) mod auth;
mod cache;
mod submit;

use async_std::channel::Sender;

use crate::{
	declaration::{
		Entry, Source,
		value::{self, Value},
	},
	meta_brainz,
};

use super::channel::{Action, Status, report};

pub(super) type Rating = (Source, Value);

pub(super) struct Pending {
	client: auth::Client,
	submitted: cache::Submitted,
	pub rating: Vec<Rating>,
}

pub(super) fn pending(list: &[Entry]) -> hmerr::Result<Option<Pending>> {
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

pub(super) fn acquire(pending: &Pending) -> hmerr::Result<Option<String>> {
	auth::acquire(&pending.client)
}

pub(super) async fn sync(bearer: String, pending: Pending, tx: Sender<Status>) {
	let Pending {
		mut submitted,
		rating,
		..
	} = pending;

	for chunk in rating.chunks(submit::CHUNK) {
		let sent = submit::submit(&bearer, chunk).and_then(|()| {
			submitted.extend(chunk.iter().copied());
			cache::write(&submitted)
		});

		let gave_up = sent
			.as_ref()
			.err()
			.is_some_and(|e| meta_brainz::gave_up(&**e));

		report(
			&tx,
			Action::SubmitRating(chunk.len()),
			sent.map_err(|e| e.to_string()),
		)
		.await;

		if gave_up {
			return;
		}
	}
}

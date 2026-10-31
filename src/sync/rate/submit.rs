use std::fmt::Write;

use miette::miette;

use crate::{meta_brainz::Sent, music_brainz};

use super::Rating;

pub(super) const CHUNK: usize = 200;

const ENDPOINT: &str = "https://musicbrainz.org/ws/2/rating";

const CONTENT_TYPE: &str = "application/xml; charset=utf-8";

const FAILURE: &str = "failed to submit rating";

pub(super) fn submit(bearer: &str, rating: &[Rating]) -> miette::Result<()> {
	let Sent {
		status,
		body: detail,
	} = music_brainz::post(
		&format!("{ENDPOINT}?client={client}", client = listen_agent::CLIENT),
		|request| {
			request
				.header("content-type", CONTENT_TYPE)
				.header("authorization", format!("Bearer {bearer}"))
				.send(body(rating))
		},
		FAILURE,
	)?;

	if !status.is_success() {
		return Err(miette!(
			"musicbrainz refused rating submission ({status})\n{detail}"
		));
	}

	Ok(())
}

fn body(rating: &[Rating]) -> String {
	let mut recording = String::new();
	for (source, value) in rating {
		let _ = write!(
			recording,
			"<recording id=\"{source}\"><user-rating>{value}</user-rating></recording>",
		);
	}

	format!(
		"<?xml version=\"1.0\" encoding=\"UTF-8\"?><metadata xmlns=\"http://musicbrainz.org/ns/mmd-2.0#\"><recording-list>{recording}</recording-list></metadata>"
	)
}

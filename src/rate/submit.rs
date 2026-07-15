use std::{fmt::Write, time::Duration};

use ansi::abbrev::{B, D, R};
use hmerr::ge;

use crate::{MUSIC_BRAINZ_CLIENT, MUSIC_BRAINZ_USER_AGENT};

use super::{Rating, agent};

pub(super) const CHUNK: usize = 200;
pub(super) const RATE_LIMIT: Duration = Duration::from_secs(1);

const ENDPOINT: &str = "https://musicbrainz.org/ws/2/rating";

const CONTENT_TYPE: &str = "application/xml; charset=utf-8";

pub(super) fn submit(bearer: &str, rating: &[Rating]) -> hmerr::Result<()> {
	let mut response = agent::build()
		.post(format!("{ENDPOINT}?client={MUSIC_BRAINZ_CLIENT}"))
		.header("user-agent", MUSIC_BRAINZ_USER_AGENT)
		.header("content-type", CONTENT_TYPE)
		.header("authorization", format!("Bearer {bearer}"))
		.send(body(rating))
		.map_err(|e| ge!(format!("{R}failed to submit rating{D}\n{e}")))?;

	let status = response.status();
	if !status.is_success() {
		let detail = response.body_mut().read_to_string().unwrap_or_default();
		return Err(ge!(format!(
			"{R}musicbrainz refused rating submission{D} ({B}{status}{D})\n{detail}"
		))
		.into());
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

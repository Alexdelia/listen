use ansi::abbrev::{D, R};

use crate::{declaration::Source, listen_brainz};

const PATH: &str = "popularity/recording";
pub(super) const BATCH: usize = 1000;

pub(super) fn popularity(recording: &[Source]) -> hmerr::Result<String> {
	let body = serde_json::json!({
		"recording_mbids": recording.iter().map(ToString::to_string).collect::<Vec<_>>(),
	});

	Ok(listen_brainz::post(
		PATH,
		&body,
		&format!(
			"{R}failed to fetch the popularity of {count} recording{D}",
			count = recording.len()
		),
	)?
	.body)
}

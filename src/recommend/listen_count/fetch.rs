use ansi::abbrev::{D, R};
use hmerr::ge;

use crate::{declaration::Source, meta_brainz};

const URL: &str = "https://api.listenbrainz.org/1/popularity/recording";
pub(super) const BATCH: usize = 1000;

pub(super) fn popularity(recording: &[Source]) -> hmerr::Result<String> {
	let body = serde_json::json!({
		"recording_mbids": recording.iter().map(ToString::to_string).collect::<Vec<_>>(),
	});

	meta_brainz::block_ready();

	let mut response = ureq::post(URL).send_json(&body).map_err(|e| {
		ge!(format!(
			"{R}failed to fetch the popularity of {count} recording{D}\n{e}",
			count = recording.len()
		))
	})?;

	response
		.body_mut()
		.read_to_string()
		.map_err(|e| ge!(format!("{R}failed to read recording popularity{D}\n{e}")).into())
}

use crate::{declaration::Source, listen_brainz};

const PATH: &str = "popularity/recording";
pub(super) const BATCH: usize = 1000;

pub(super) fn popularity(recording: &[Source]) -> miette::Result<String> {
	let body = serde_json::json!({
		"recording_mbids": recording.iter().map(ToString::to_string).collect::<Vec<_>>(),
	});

	Ok(listen_brainz::post(
		PATH,
		&body,
		&format!(
			"failed to fetch the popularity of {count} recording",
			count = recording.len()
		),
	)?
	.body)
}

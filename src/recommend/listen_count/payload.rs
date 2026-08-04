use ansi::abbrev::{D, R};
use hmerr::ge;
use serde::Deserialize;

use crate::declaration::Source;

#[derive(Deserialize)]
pub(super) struct Popularity {
	pub recording_mbid: Source,
	#[serde(default)]
	pub total_listen_count: Option<u64>,
	#[serde(default)]
	pub total_user_count: Option<u64>,
}

pub(super) fn popularity(body: &str) -> hmerr::Result<Vec<Popularity>> {
	serde_json::from_str(body)
		.map_err(|e| ge!(format!("{R}failed to parse recording popularity{D}\n{e}")).into())
}

#[cfg(test)]
mod tests {
	use super::*;

	const BODY: &str = r#"[
		{
			"recording_mbid": "5ecaf4e8-c19d-4756-b697-20b8478b0c8c",
			"total_listen_count": 12791,
			"total_user_count": 1492
		},
		{
			"recording_mbid": "aaaaaaaa-c19d-4756-b697-20b8478b0c8c",
			"total_listen_count": null,
			"total_user_count": null
		}
	]"#;

	#[test]
	fn every_recording_keeps_its_counts() {
		let found = popularity(BODY).unwrap_or_default();

		assert_eq!(
			found
				.iter()
				.map(|p| (p.total_listen_count, p.total_user_count))
				.collect::<Vec<_>>(),
			vec![(Some(12791), Some(1492)), (None, None)]
		);
	}

	#[test]
	fn a_broken_body_is_an_error() {
		assert!(popularity("not json").is_err());
	}
}

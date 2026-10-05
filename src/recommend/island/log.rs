use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{cache, declaration::Source};

const FILE: &str = "recommended.jsonl";

pub(super) fn path() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(FILE))
}

#[derive(Serialize, Deserialize)]
pub(super) struct Entry {
	pub mbid: Source,
	pub island: String,
	pub member: usize,
	pub score: f32,
	pub backer: u32,
	pub listener: u32,
	pub plays: u64,
	pub popularity_damp: f32,
	#[serde(default)]
	pub allow_known_artist: bool,
	pub granularity: f64,
	pub stay: bool,
	pub shown_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
	use super::*;

	fn entry() -> Entry {
		Entry {
			mbid: Source::from_bytes([7; 16]),
			island: "touhou / speedcore".to_string(),
			member: 30,
			score: 1993.2,
			backer: 51,
			listener: 671,
			plays: 7083,
			popularity_damp: 1.0 / 3.0,
			allow_known_artist: false,
			granularity: 1.0,
			stay: true,
			shown_at: DateTime::from_timestamp(0, 0).unwrap_or_default(),
		}
	}

	#[test]
	fn an_entry_survives_a_round_trip() {
		let line = serde_json::to_string(&entry()).unwrap_or_default();
		let read: Entry = serde_json::from_str(&line).unwrap_or_else(|_| entry());

		assert_eq!(read.mbid, entry().mbid);
		assert_eq!(read.island, entry().island);
		assert!((read.score - entry().score).abs() < f32::EPSILON);
	}

	#[test]
	fn a_line_logged_before_known_artists_could_be_allowed_still_reads_as_excluding_them() {
		let line = r#"{"mbid":"0ad8a5e9-3a8d-43a9-bd65-aedb04a95e99","island":"touhou","member":66,"score":4.746328,"backer":56,"listener":3182,"plays":38635,"popularity_damp":0.0,"granularity":1.5,"stay":true,"shown_at":"2026-10-04T16:06:32.427518647Z"}"#;

		let read: Option<Entry> = serde_json::from_str(line).ok();

		assert!(read.is_some_and(|entry| !entry.allow_known_artist));
	}

	#[test]
	fn the_island_is_logged_by_name_because_its_number_is_not_stable() {
		let line = serde_json::to_string(&entry()).unwrap_or_default();

		assert!(line.contains("touhou / speedcore"), "{line}");
	}
}

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use listen_cache::text;
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
	pub granularity: f64,
	pub stay: bool,
	pub shown_at: DateTime<Utc>,
}

pub(super) fn append(path: &Path, entry: &Entry) -> hmerr::Result<()> {
	text::append(path, &serde_json::to_string(entry)?)
}

#[cfg(test)]
mod tests {
	use std::fs;

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
	fn the_island_is_logged_by_name_because_its_number_is_not_stable() {
		let line = serde_json::to_string(&entry()).unwrap_or_default();

		assert!(line.contains("touhou / speedcore"), "{line}");
	}

	fn read(path: &Path) -> Vec<Entry> {
		fs::read_to_string(path)
			.unwrap_or_default()
			.lines()
			.filter_map(|line| serde_json::from_str(line).ok())
			.collect()
	}

	#[test]
	fn an_appended_recommendation_lands_as_one_json_line() {
		let path = std::env::temp_dir().join("declarative_listen_log_append.jsonl");
		let _ = fs::remove_file(&path);

		let _ = append(&path, &entry());
		let _ = append(&path, &entry());

		let logged = read(&path);

		assert_eq!(logged.len(), 2);
		assert_eq!(logged[0].mbid, entry().mbid);
		let _ = fs::remove_file(&path);
	}

	#[test]
	fn the_log_is_only_ever_appended_to() {
		let path = std::env::temp_dir().join("declarative_listen_log_keep.jsonl");
		let _ = fs::write(&path, "already here\n");

		let _ = append(&path, &entry());
		let content = fs::read_to_string(&path).unwrap_or_default();

		assert!(content.starts_with("already here"), "{content}");
		assert_eq!(read(&path).len(), 1);
		let _ = fs::remove_file(&path);
	}
}

use std::{
	fs::{self, OpenOptions},
	io::Write,
	path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use hmerr::ioe;
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
	pub plays: u32,
	pub alpha: f32,
	pub resolution: f64,
	pub stay: bool,
	pub shown_at: DateTime<Utc>,
}

pub(super) fn shown(path: &Path) -> hmerr::Result<Vec<Source>> {
	if !path.exists() {
		return Ok(Vec::new());
	}

	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(content
		.lines()
		.filter_map(|line| serde_json::from_str::<Entry>(line).ok())
		.map(|entry| entry.mbid)
		.collect())
}

pub(super) fn append(path: &Path, entry: &Entry) -> hmerr::Result<()> {
	cache::prepare(path)?;

	let line = serde_json::to_string(entry)?;

	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(path)
		.map_err(|e| ioe!(path.to_string_lossy(), e))?;

	writeln!(file, "{line}").map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
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
			plays: 7083,
			alpha: 0.6,
			resolution: 1.0,
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

	#[test]
	fn an_appended_recommendation_comes_back_as_shown() {
		let path = std::env::temp_dir().join("declarative_listen_log_test.jsonl");
		let _ = fs::remove_file(&path);

		assert!(shown(&path).unwrap_or_default().is_empty());
		let _ = append(&path, &entry());

		assert_eq!(shown(&path).unwrap_or_default(), vec![entry().mbid]);
		let _ = fs::remove_file(&path);
	}

	#[test]
	fn a_malformed_line_does_not_hide_the_rest() {
		let path = std::env::temp_dir().join("declarative_listen_log_broken.jsonl");
		let _ = fs::write(&path, "not json\n");
		let _ = append(&path, &entry());

		assert_eq!(shown(&path).unwrap_or_default(), vec![entry().mbid]);
		let _ = fs::remove_file(&path);
	}
}

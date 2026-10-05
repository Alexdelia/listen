use std::path::PathBuf;

use listen_cache::text;
use serde::Serialize;

pub(super) enum Log {
	File(PathBuf),
	Off,
}

impl Log {
	pub(super) fn append(&self, entry: &impl Serialize) -> hmerr::Result<()> {
		match self {
			Self::File(path) => text::append(path, &serde_json::to_string(entry)?),
			Self::Off => Ok(()),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::Path};

	use serde::Serialize;

	use super::*;

	#[derive(Serialize)]
	struct Shown {
		mbid: u8,
	}

	fn read(path: &Path) -> Vec<String> {
		fs::read_to_string(path)
			.unwrap_or_default()
			.lines()
			.map(str::to_string)
			.collect()
	}

	#[test]
	fn an_entry_appended_to_a_file_lands_as_one_json_line() {
		let path = std::env::temp_dir().join("declarative_listen_log_append.jsonl");
		let _ = fs::remove_file(&path);
		let log = Log::File(path.clone());

		let _ = log.append(&Shown { mbid: 1 });
		let _ = log.append(&Shown { mbid: 2 });

		assert_eq!(read(&path), vec![r#"{"mbid":1}"#, r#"{"mbid":2}"#]);
		let _ = fs::remove_file(&path);
	}

	#[test]
	fn the_file_is_only_ever_appended_to() {
		let path = std::env::temp_dir().join("declarative_listen_log_keep.jsonl");
		let _ = fs::write(&path, "already here\n");

		let _ = Log::File(path.clone()).append(&Shown { mbid: 1 });

		assert_eq!(
			read(&path).first().map(String::as_str),
			Some("already here")
		);
		assert_eq!(read(&path).len(), 2);
		let _ = fs::remove_file(&path);
	}

	#[test]
	fn logging_off_accepts_every_entry() {
		assert!(Log::Off.append(&Shown { mbid: 1 }).is_ok());
	}
}

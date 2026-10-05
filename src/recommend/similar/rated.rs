use std::{collections::HashSet, path::PathBuf};

use listen_cache::text;
use serde::Deserialize;

use crate::declaration::{Entry, Q, Source};

#[derive(Deserialize)]
struct Shown {
	mbid: Source,
}

pub(super) fn of(entry: &[Entry], log: &[PathBuf]) -> hmerr::Result<Vec<(Source, Q)>> {
	let mut shown = HashSet::new();

	for path in log {
		let Some(content) = text::read(path)? else {
			continue;
		};

		shown.extend(
			content
				.lines()
				.filter_map(|line| serde_json::from_str::<Shown>(line).ok())
				.map(|shown| shown.mbid),
		);
	}

	Ok(entry
		.iter()
		.filter(|entry| shown.contains(&entry.s))
		.map(|entry| (entry.s, entry.q))
		.collect())
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;

	fn entry(byte: u8, q: Q) -> Entry {
		Entry {
			s: Source::from_bytes([byte; 16]),
			q,
			playlist: Vec::new(),
		}
	}

	fn log(name: &str, line: &[String]) -> PathBuf {
		let path = std::env::temp_dir().join(format!("declarative_listen_rated_{name}.jsonl"));
		let _ = fs::write(&path, line.join("\n"));
		path
	}

	fn shown(byte: u8) -> String {
		format!(
			r#"{{"mbid":"{}","island":"x"}}"#,
			Source::from_bytes([byte; 16])
		)
	}

	#[test]
	fn a_shown_pick_now_declared_is_rated_at_its_declared_q() {
		let island = log("island", &[shown(1), shown(2)]);
		let similar = log("similar", &[shown(3)]);

		let rated = of(
			&[entry(1, 3), entry(3, 0), entry(9, 4)],
			&[island.clone(), similar.clone()],
		)
		.unwrap_or_default();

		assert_eq!(
			rated,
			vec![
				(Source::from_bytes([1; 16]), 3),
				(Source::from_bytes([3; 16]), 0)
			]
		);
		let _ = fs::remove_file(island);
		let _ = fs::remove_file(similar);
	}

	#[test]
	fn a_missing_log_or_an_unreadable_line_is_skipped() {
		let broken = log("broken", &["not json".to_string(), shown(1)]);
		let missing = std::env::temp_dir().join("declarative_listen_rated_missing.jsonl");
		let _ = fs::remove_file(&missing);

		let rated = of(&[entry(1, 2)], &[broken.clone(), missing]).unwrap_or_default();

		assert_eq!(rated, vec![(Source::from_bytes([1; 16]), 2)]);
		let _ = fs::remove_file(broken);
	}
}

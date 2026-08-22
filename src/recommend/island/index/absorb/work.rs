use std::{
	fs,
	path::{Path, PathBuf},
};

use hmerr::ioe;
use serde::{Deserialize, Serialize};

use super::super::{
	open::{Gap, Meta},
	partial, work,
};

pub(super) use work::{publish, release};

const DIR: &str = "absorb";
const FROM: &str = "from";
const REACH: &str = "reach.json";
const FORMAT: u32 = 1;

const DELTA: &str = "delta";
pub(super) const LIBRARY: &str = "library";
pub(super) const ARTIST: &str = "artist";

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Reach {
	pub covered: String,
	pub gap: Vec<Gap>,
	pub absorbed: u32,
}

pub(super) fn open(dir: &Path, from: &str) -> hmerr::Result<PathBuf> {
	work::opened(dir, DIR, FROM, &format!("{FORMAT} {from}"))
}

pub(super) fn reach(work: &Path, meta: &Meta) -> Reach {
	held(work).unwrap_or_else(|| Reach {
		covered: meta.covered().to_string(),
		gap: meta.gap.clone(),
		absorbed: 0,
	})
}

pub(super) fn reached(work: &Path, reach: &Reach) -> hmerr::Result<()> {
	let content = serde_json::to_string(reach)?;

	partial::write(&work.join(REACH), |partial| {
		fs::write(partial, &content).map_err(|e| ioe!(partial.to_string_lossy(), e))?;

		Ok(())
	})
}

pub(super) fn delta(work: &Path, of: &str) -> PathBuf {
	work.join(DELTA).join(of)
}

pub(super) fn read(work: &Path, of: &str) -> String {
	format!(
		"read_parquet('{delta}/*.parquet')",
		delta = delta(work, of).display()
	)
}

pub(super) fn folded(work: &Path, of: &str) -> bool {
	let Ok(read) = fs::read_dir(delta(work, of)) else {
		return false;
	};

	read.filter_map(Result::ok)
		.any(|entry| entry.path().extension().is_some_and(|ext| ext == "parquet"))
}

fn held(work: &Path) -> Option<Reach> {
	serde_json::from_str(&fs::read_to_string(work.join(REACH)).ok()?).ok()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_absorb_work_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn meta(covered: Option<&str>) -> Meta {
		Meta {
			built: "2026-08-15".to_string(),
			dump: "2026-07-12 00:00:04.001868+00:00".to_string(),
			own: Some(1),
			reached: covered.map(str::to_string),
			gap: Vec::new(),
			absorbed: 0,
			user: 5,
			recording: 35,
			user_listen: 200,
		}
	}

	#[test]
	fn a_fresh_run_starts_where_the_index_left_off() {
		let dir = dir("fresh");
		let work = open(&dir, "2026-07-12 00:00:04.001868+00:00").unwrap_or_default();

		assert_eq!(
			reach(&work, &meta(None)).covered,
			"2026-07-12 00:00:04.001868+00:00"
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_index_coverage_wins_over_the_dump_it_was_built_from() {
		let dir = dir("reached");
		let work = open(&dir, "2026-08-01 00:00:03.000000+00:00").unwrap_or_default();

		assert_eq!(
			reach(&work, &meta(Some("2026-08-01 00:00:03.000000+00:00"))).covered,
			"2026-08-01 00:00:03.000000+00:00"
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_run_that_died_resumes_where_its_last_fold_reached() {
		let dir = dir("resume");
		let from = "2026-07-12 00:00:04.001868+00:00";
		let work = open(&dir, from).unwrap_or_default();

		let _ = reached(
			&work,
			&Reach {
				covered: "2026-08-10 00:00:02.000000+00:00".to_string(),
				gap: vec![Gap {
					from: from.to_string(),
					to: "2026-07-23 00:00:03.690928+00:00".to_string(),
				}],
				absorbed: 18,
			},
		);

		let again = open(&dir, from).unwrap_or_default();
		let reach = reach(&again, &meta(None));

		assert_eq!(reach.covered, "2026-08-10 00:00:02.000000+00:00");
		assert_eq!(reach.absorbed, 18);
		assert_eq!(reach.gap.len(), 1);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_republished_index_throws_away_what_was_folded_against_the_old_one() {
		let dir = dir("stale");
		let work = open(&dir, "2026-07-12 00:00:04.001868+00:00").unwrap_or_default();
		let _ = fs::create_dir_all(delta(&work, LIBRARY));
		let _ = fs::write(delta(&work, LIBRARY).join("0.parquet"), b"folded");

		let again = open(&dir, "2026-08-22 00:00:02.641933+00:00").unwrap_or_default();

		assert!(!folded(&again, LIBRARY));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn nothing_folded_is_nothing_to_merge() {
		let dir = dir("empty");
		let work = open(&dir, "2026-07-12 00:00:04.001868+00:00").unwrap_or_default();

		assert!(!folded(&work, LIBRARY));

		let _ = fs::create_dir_all(delta(&work, LIBRARY));
		let _ = fs::write(delta(&work, LIBRARY).join("0.parquet"), b"folded");

		assert!(folded(&work, LIBRARY));
		let _ = fs::remove_dir_all(&dir);
	}
}

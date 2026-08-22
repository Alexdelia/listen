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
const MERGE: &str = "merge";
const AT: &str = "at";
pub(super) const LIBRARY: &str = "library";
pub(super) const ARTIST: &str = "artist";

pub(super) struct Merge {
	pub index: PathBuf,
	pub work: PathBuf,
	pub into: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Reach {
	pub covered: String,
	pub gap: Vec<Gap>,
	pub absorbed: u32,
}

pub(super) fn open(dir: &Path, from: &str) -> hmerr::Result<PathBuf> {
	work::opened(dir, DIR, FROM, &format!("{FORMAT} {from}"))
}

pub(super) fn merging(dir: &Path, work: &Path, covered: &str) -> hmerr::Result<Merge> {
	Ok(Merge {
		index: dir.to_path_buf(),
		work: work.to_path_buf(),
		into: work::opened(work, MERGE, AT, covered)?,
	})
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
	use super::{super::super::open::RECORDING, *};

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
	fn a_dump_folded_after_a_merge_staged_a_part_throws_that_part_away() {
		let dir = dir("folded_after");
		let work = open(&dir, "2026-07-12 00:00:04.001868+00:00").unwrap_or_default();
		let merge = merging(&dir, &work, "2026-08-10 00:00:02.000000+00:00")
			.unwrap_or_else(|_| unreachable!());
		let staged = merge.into.join(RECORDING);
		let _ = fs::write(&staged, b"merged");

		let again = merging(&dir, &work, "2026-08-11 00:00:02.000000+00:00")
			.unwrap_or_else(|_| unreachable!());

		assert_eq!(again.into, merge.into);
		assert!(!staged.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_merge_retried_over_the_same_fold_keeps_what_it_staged() {
		let dir = dir("retried");
		let work = open(&dir, "2026-07-12 00:00:04.001868+00:00").unwrap_or_default();
		let covered = "2026-08-10 00:00:02.000000+00:00";
		let merge = merging(&dir, &work, covered).unwrap_or_else(|_| unreachable!());
		let staged = merge.into.join(RECORDING);
		let _ = fs::write(&staged, b"merged");

		let _ = merging(&dir, &work, covered);

		assert!(staged.exists());
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

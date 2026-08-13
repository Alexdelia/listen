use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

const DIR: &str = "build";
const STAMP: &str = "dump";

pub(super) fn open(dir: &Path, dump: &str) -> hmerr::Result<PathBuf> {
	let work = dir.join(DIR);

	if built_from(&work).as_deref() != Some(dump) {
		discard(&work)?;
	}

	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;

	let stamp = work.join(STAMP);
	fs::write(&stamp, dump).map_err(|e| ioe!(stamp.to_string_lossy(), e))?;

	Ok(work)
}

pub(super) fn release(work: &Path) {
	if discard(work).is_ok() {
		return;
	}

	println!(
		"{F}the index is built, but its work directory stayed at {B}{work}{D}\n\
		{F}delete it to reclaim the space{D}",
		work = work.display()
	);
}

fn built_from(work: &Path) -> Option<String> {
	fs::read_to_string(work.join(STAMP))
		.ok()
		.map(|dump| dump.trim().to_string())
}

fn discard(work: &Path) -> hmerr::Result<()> {
	if !work.is_dir() {
		return Ok(());
	}

	fs::remove_dir_all(work).map_err(|e| ioe!(work.to_string_lossy(), e))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_work_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn partial(work: &Path) -> PathBuf {
		let partial = work.join("listen").join("0.parquet");
		let _ = fs::create_dir_all(partial.parent().unwrap_or(work));
		let _ = fs::write(&partial, b"partial");

		partial
	}

	#[test]
	fn the_same_dump_keeps_what_a_previous_run_already_scanned() {
		let dir = dir("resume");
		let work = open(&dir, "listenbrainz-dump-2593-full").unwrap_or_default();
		let partial = partial(&work);

		let again = open(&dir, "listenbrainz-dump-2593-full").unwrap_or_default();

		assert_eq!(again, work);
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_throws_the_previous_scan_away() {
		let dir = dir("stale");
		let work = open(&dir, "listenbrainz-dump-2593-full").unwrap_or_default();
		let partial = partial(&work);

		let _ = open(&dir, "listenbrainz-dump-2600-full");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_work_directory_left_by_an_older_version_is_thrown_away() {
		let dir = dir("unstamped");
		let work = dir.join(DIR);
		let partial = partial(&work);

		let _ = open(&dir, "listenbrainz-dump-2593-full");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn releasing_leaves_nothing_behind() {
		let dir = dir("release");
		let work = open(&dir, "listenbrainz-dump-2593-full").unwrap_or_default();
		partial(&work);

		release(&work);

		assert!(!work.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn releasing_a_work_directory_that_is_already_gone_says_nothing() {
		let dir = dir("absent");

		release(&dir.join(DIR));
		let _ = fs::remove_dir_all(&dir);
	}
}

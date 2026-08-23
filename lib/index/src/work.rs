use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

use super::{
	index::{
		self, Meta,
		layout::{RECORDING, RECORDING_ARTIST, RECORDING_LISTENER, USER_LISTEN, USER_STAT},
	},
	keep, progress,
};

pub(super) const LIBRARY: &str = "library";
pub(super) const ARTIST: &str = "artist";
pub(super) const STAT: &str = "stat";

pub(super) const fn published() -> [&'static str; 5] {
	[
		RECORDING,
		RECORDING_ARTIST,
		RECORDING_LISTENER,
		USER_STAT,
		USER_LISTEN,
	]
}

pub(super) fn publish(work: &Path, dir: &Path, meta: &Meta) -> hmerr::Result<()> {
	index::meta::forget(dir)?;

	for part in published() {
		let built = work.join(part);
		let into = dir.join(part);

		keep::removed(&into)?;
		fs::rename(&built, &into).map_err(|e| ioe!(into.to_string_lossy(), e))?;
	}

	index::meta::write(dir, meta)
}

pub(super) fn release(work: &Path) {
	if keep::discard(work).is_ok() {
		return;
	}

	progress::say(format!(
		"{F}work directory kept at {B}{work}{D}",
		work = work.display()
	));
}

pub(super) fn stamped(work: &Path, of: &str) -> Option<String> {
	fs::read_to_string(work.join(of))
		.ok()
		.map(|stamp| stamp.trim().to_string())
}

pub(super) fn stamp(work: &Path, of: &str, value: &str) -> hmerr::Result<()> {
	let path = work.join(of);
	fs::write(&path, value).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

pub(super) fn opened(
	dir: &Path,
	name: &str,
	stamp_of: &str,
	value: &str,
) -> hmerr::Result<PathBuf> {
	let work = dir.join(name);

	if stamped(&work, stamp_of).as_deref() != Some(value) {
		keep::removed(&work)?;
	}

	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;
	stamp(&work, stamp_of, value)?;

	Ok(work)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn staged(work: &Path) {
		for part in [RECORDING, RECORDING_ARTIST, RECORDING_LISTENER, USER_STAT] {
			let _ = fs::write(work.join(part), b"fresh");
		}

		let listen = work.join(USER_LISTEN);
		let _ = fs::create_dir_all(&listen);
		let _ = fs::write(listen.join("0.parquet"), b"fresh");
	}

	fn meta() -> Meta {
		Meta {
			built: "2026-08-15".to_string(),
			dump: "20260712-000004".to_string(),
			own: Some(1),
			reached: None,
			gap: Vec::new(),
			absorbed: 0,
			user: 5,
			recording: 35,
			user_listen: 200,
		}
	}

	#[test]
	fn a_staged_part_is_named_what_an_earlier_run_wrote_on_disk() {
		assert_eq!(LIBRARY, "library");
		assert_eq!(ARTIST, "artist");
		assert_eq!(STAT, "stat");
	}

	#[test]
	fn publishing_puts_every_staged_part_in_place() {
		let dir = crate::scratch::of("index_work", "publish");
		let held = dir.join(USER_STAT);
		let _ = fs::write(&held, b"held");
		let work = opened(&dir, "staging", "stamp", "one").unwrap_or_default();
		staged(&work);

		assert!(publish(&work, &dir, &meta()).is_ok());

		assert_eq!(fs::read(&held).unwrap_or_default(), b"fresh");
		for part in published() {
			assert!(dir.join(part).exists(), "{part}");
			assert!(!work.join(part).exists(), "{part}");
		}
		assert!(dir.join(index::layout::META).exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_publish_that_cannot_finish_leaves_no_meta_vouching_for_a_half_swapped_index() {
		let dir = crate::scratch::of("index_work", "torn");
		let _ = fs::write(dir.join(index::layout::META), b"{}");
		let work = opened(&dir, "staging", "stamp", "one").unwrap_or_default();

		assert!(publish(&work, &dir, &meta()).is_err());

		assert!(!dir.join(index::layout::META).exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_same_stamp_keeps_what_a_previous_run_left() {
		let dir = crate::scratch::of("index_work", "resume");
		let work = opened(&dir, "staging", "stamp", "one").unwrap_or_default();
		let left = work.join("left");
		let _ = fs::write(&left, b"partial");

		let again = opened(&dir, "staging", "stamp", "one").unwrap_or_default();

		assert_eq!(again, work);
		assert!(left.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_stamp_throws_away_what_a_previous_run_left() {
		let dir = crate::scratch::of("index_work", "stale");
		let work = opened(&dir, "staging", "stamp", "one").unwrap_or_default();
		let left = work.join("left");
		let _ = fs::write(&left, b"partial");

		let _ = opened(&dir, "staging", "stamp", "two");

		assert!(!left.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn releasing_leaves_nothing_behind() {
		let dir = crate::scratch::of("index_work", "release");
		let work = opened(&dir, "staging", "stamp", "one").unwrap_or_default();

		release(&work);

		assert!(!work.exists());
		let _ = fs::remove_dir_all(&dir);
	}
}

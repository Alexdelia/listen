use std::path::{Path, PathBuf};

use super::super::{
	index::layout::{RECORDING_LISTENER, USER_LISTEN, USER_STAT},
	keep, work,
};

pub(super) use work::{ARTIST, LIBRARY, STAT, publish, release};

const DIR: &str = "build";

const DUMP: &str = "dump";
const EXCLUDED: &str = "excluded";
const FORMAT: u32 = 4;

pub(super) fn open(dir: &Path, dump: &str) -> hmerr::Result<PathBuf> {
	work::opened(dir, DIR, DUMP, &format!("{FORMAT} {dump}"))
}

pub(super) fn exclude(work: &Path, own: u32) -> hmerr::Result<()> {
	let own = &own.to_string();

	if work::stamped(work, EXCLUDED).as_deref() != Some(own.as_str()) {
		for part in pooled() {
			keep::removed(&work.join(part))?;
		}
	}

	work::stamp(work, EXCLUDED, own)
}

const fn pooled() -> [&'static str; 4] {
	[STAT, USER_STAT, USER_LISTEN, RECORDING_LISTENER]
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;

	fn shard(under: &Path, name: &str) -> PathBuf {
		let shard = under.join(name).join("0.parquet");
		if let Some(parent) = shard.parent() {
			let _ = fs::create_dir_all(parent);
		}
		let _ = fs::write(&shard, b"partial");

		shard
	}

	fn partial(work: &Path) -> PathBuf {
		shard(work, LIBRARY)
	}

	fn stat(dir: &Path) -> PathBuf {
		let stat = dir.join(USER_STAT);
		let _ = fs::write(&stat, b"built");

		stat
	}

	#[test]
	fn the_same_dump_keeps_what_a_previous_run_already_scanned() {
		let dir = crate::scratch::of("work", "resume");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);

		let again = open(&dir, "20260712-000004").unwrap_or_default();

		assert_eq!(again, work);
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_throws_the_previous_scan_away() {
		let dir = crate::scratch::of("work", "stale");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);

		let _ = open(&dir, "20260809-000003");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_work_directory_left_by_an_older_version_is_thrown_away() {
		let dir = crate::scratch::of("work", "unstamped");
		let work = dir.join(DIR);
		let partial = partial(&work);

		let _ = open(&dir, "20260712-000004");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_declaration_never_reaches_the_scan_so_editing_it_throws_nothing_away() {
		let dir = crate::scratch::of("work", "declaration");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);
		let artist = shard(&work, ARTIST);
		let listen = shard(&dir, USER_LISTEN);
		let stat = stat(&dir);

		let _ = open(&dir, "20260712-000004");

		assert!(partial.exists());
		assert!(artist.exists());
		assert!(listen.exists());
		assert!(stat.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn resuming_under_the_same_own_listener_keeps_what_was_pooled_for_it() {
		let dir = crate::scratch::of("work", "same_own");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let _ = exclude(&work, 1);
		let stat = shard(&work, STAT);
		let listen = shard(&work, USER_LISTEN);

		let _ = exclude(&work, 1);

		assert!(stat.exists());
		assert!(listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_own_listener_throws_away_what_the_previous_one_pooled() {
		let dir = crate::scratch::of("work", "other_own");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);
		let _ = exclude(&work, 1);
		let stat = shard(&work, STAT);
		let listen = shard(&work, USER_LISTEN);

		let _ = exclude(&work, 2);

		assert!(!stat.exists());
		assert!(!listen.exists());
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_leaves_the_published_index_alone() {
		let dir = crate::scratch::of("work", "published");
		let _ = open(&dir, "20260712-000004");
		let listen = shard(&dir, USER_LISTEN);
		let stat = stat(&dir);

		let _ = open(&dir, "20260809-000003");

		assert!(listen.exists());
		assert!(stat.exists());
		let _ = fs::remove_dir_all(&dir);
	}
}

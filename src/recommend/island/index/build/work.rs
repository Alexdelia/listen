use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

use super::super::{
	keep,
	open::{USER_LISTEN, USER_STAT},
};

const DIR: &str = "build";
const DUMP: &str = "dump";
const FORMAT: u32 = 2;

pub(super) fn open(dir: &Path, dump: &str) -> hmerr::Result<PathBuf> {
	let work = dir.join(DIR);
	let dump = &format!("{FORMAT} {dump}");

	if stamped(&work, DUMP).as_deref() != Some(dump.as_str()) {
		discard_unusable(&work)?;
		for stale in published(dir) {
			discard_unusable(&stale)?;
		}
	}

	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;
	stamp(&work, DUMP, dump)?;

	Ok(work)
}

pub(super) fn release(work: &Path) {
	if keep::discard(work).is_ok() {
		return;
	}

	println!(
		"{F}the index is built, but its work directory stayed at {B}{work}{D}\n\
		{F}delete it to reclaim the space{D}",
		work = work.display()
	);
}

fn published(dir: &Path) -> [PathBuf; 2] {
	[dir.join(USER_LISTEN), dir.join(USER_STAT)]
}

fn stamped(work: &Path, of: &str) -> Option<String> {
	fs::read_to_string(work.join(of))
		.ok()
		.map(|stamp| stamp.trim().to_string())
}

fn stamp(work: &Path, of: &str, value: &str) -> hmerr::Result<()> {
	let path = work.join(of);
	fs::write(&path, value).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn discard_unusable(path: &Path) -> hmerr::Result<()> {
	if path.is_dir() {
		fs::remove_dir_all(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	} else if path.is_file() {
		fs::remove_file(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::{
		super::{artist, library},
		*,
	};

	fn dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_work_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn shard(under: &Path, name: &str) -> PathBuf {
		let shard = under.join(name).join("0.parquet");
		if let Some(parent) = shard.parent() {
			let _ = fs::create_dir_all(parent);
		}
		let _ = fs::write(&shard, b"partial");

		shard
	}

	fn partial(work: &Path) -> PathBuf {
		shard(work, library::NAME)
	}

	fn stat(dir: &Path) -> PathBuf {
		let stat = dir.join(USER_STAT);
		let _ = fs::write(&stat, b"built");

		stat
	}

	#[test]
	fn the_same_dump_keeps_what_a_previous_run_already_scanned() {
		let dir = dir("resume");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);

		let again = open(&dir, "20260712-000004").unwrap_or_default();

		assert_eq!(again, work);
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_throws_the_previous_scan_away() {
		let dir = dir("stale");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);

		let _ = open(&dir, "20260809-000003");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_work_directory_left_by_an_older_version_is_thrown_away() {
		let dir = dir("unstamped");
		let work = dir.join(DIR);
		let partial = partial(&work);

		let _ = open(&dir, "20260712-000004");

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_declaration_never_reaches_the_scan_so_editing_it_throws_nothing_away() {
		let dir = dir("declaration");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let partial = partial(&work);
		let artist = shard(&work, artist::NAME);
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
	fn another_dump_throws_away_the_published_index() {
		let dir = dir("published");
		let _ = open(&dir, "20260712-000004");
		let listen = shard(&dir, USER_LISTEN);
		let stat = stat(&dir);

		let _ = open(&dir, "20260809-000003");

		assert!(!listen.exists());
		assert!(!stat.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_released_build_cannot_vouch_for_the_published_user_listen() {
		let dir = dir("released");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		let listen = shard(&dir, USER_LISTEN);
		release(&work);

		let _ = open(&dir, "20260712-000004");

		assert!(!listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn releasing_leaves_nothing_behind() {
		let dir = dir("release");
		let work = open(&dir, "20260712-000004").unwrap_or_default();
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

use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

use super::super::{
	keep,
	open::{self, Meta, RECORDING, RECORDING_ARTIST, USER_LISTEN, USER_STAT},
	progress,
};

const DIR: &str = "build";
const DUMP: &str = "dump";
const FORMAT: u32 = 3;

pub(super) fn open(dir: &Path, dump: &str) -> hmerr::Result<PathBuf> {
	let work = dir.join(DIR);
	let dump = &format!("{FORMAT} {dump}");

	if stamped(&work, DUMP).as_deref() != Some(dump.as_str()) {
		discard_unusable(&work)?;
	}

	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;
	stamp(&work, DUMP, dump)?;

	Ok(work)
}

pub(super) fn publish(work: &Path, dir: &Path, meta: &Meta) -> hmerr::Result<()> {
	open::forget_meta(dir)?;

	for part in published() {
		let built = work.join(part);
		let into = dir.join(part);

		discard_unusable(&into)?;
		fs::rename(&built, &into).map_err(|e| ioe!(into.to_string_lossy(), e))?;
	}

	open::write_meta(dir, meta)
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

fn published() -> [&'static str; 4] {
	[RECORDING, RECORDING_ARTIST, USER_STAT, USER_LISTEN]
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
	use super::{super::board::Stage, *};

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
		shard(work, Stage::Library.title())
	}

	fn stat(dir: &Path) -> PathBuf {
		let stat = dir.join(USER_STAT);
		let _ = fs::write(&stat, b"built");

		stat
	}

	fn staged(work: &Path) {
		for part in [RECORDING, RECORDING_ARTIST, USER_STAT] {
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
			user: 5,
			recording: 35,
			user_listen: 200,
		}
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
		let artist = shard(&work, Stage::Artist.title());
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
	fn another_dump_leaves_the_published_index_alone() {
		let dir = dir("published");
		let _ = open(&dir, "20260712-000004");
		let listen = shard(&dir, USER_LISTEN);
		let stat = stat(&dir);

		let _ = open(&dir, "20260809-000003");

		assert!(listen.exists());
		assert!(stat.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn what_a_build_produces_reaches_the_index_only_when_it_publishes() {
		let dir = dir("staged");
		let stat = stat(&dir);
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		staged(&work);

		assert_eq!(fs::read(&stat).unwrap_or_default(), b"built");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn publishing_puts_every_built_part_in_place() {
		let dir = dir("publish");
		let stat = stat(&dir);
		let listen = shard(&dir, USER_LISTEN);
		let work = open(&dir, "20260712-000004").unwrap_or_default();
		staged(&work);

		assert!(publish(&work, &dir, &meta()).is_ok());

		assert_eq!(fs::read(&stat).unwrap_or_default(), b"fresh");
		assert_eq!(fs::read(&listen).unwrap_or_default(), b"fresh");
		for part in published() {
			assert!(dir.join(part).exists(), "{part}");
			assert!(!work.join(part).exists(), "{part}");
		}
		assert!(dir.join(open::META).exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_publish_that_cannot_finish_leaves_no_meta_vouching_for_a_half_swapped_index() {
		let dir = dir("torn");
		let _ = fs::write(dir.join(open::META), b"{}");
		let work = open(&dir, "20260712-000004").unwrap_or_default();

		assert!(publish(&work, &dir, &meta()).is_err());

		assert!(!dir.join(open::META).exists());
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

use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

use crate::declaration::Entry;

use super::{super::open::USER_LISTEN, artist, seed};

const DIR: &str = "build";
const DUMP: &str = "dump";
const INPUT: &str = "input";

pub(super) fn open(dir: &Path, dump: &str, declared: &[Entry]) -> hmerr::Result<PathBuf> {
	let work = dir.join(DIR);

	if stamped(&work, DUMP).as_deref() != Some(dump) {
		discard(&work)?;
	}

	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;

	let input = input(dump, declared);
	if stamped(&work, INPUT).as_deref() != Some(input.as_str()) {
		for stale in derived(dir, &work) {
			discard(&stale)?;
		}
	}

	stamp(&work, DUMP, dump)?;
	stamp(&work, INPUT, &input)?;

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

fn input(dump: &str, declared: &[Entry]) -> String {
	let mut mbid: Vec<String> = declared.iter().map(|entry| entry.s.to_string()).collect();
	mbid.sort_unstable();
	mbid.dedup();

	format!("{dump}\n{}", mbid.join("\n"))
}

fn derived(dir: &Path, work: &Path) -> [PathBuf; 3] {
	[
		work.join(seed::NAME),
		work.join(artist::NAME),
		dir.join(USER_LISTEN),
	]
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

fn discard(work: &Path) -> hmerr::Result<()> {
	if !work.is_dir() {
		return Ok(());
	}

	fs::remove_dir_all(work).map_err(|e| ioe!(work.to_string_lossy(), e))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::declaration::Source;

	use super::*;

	fn dir(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_work_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn declared(mbid: &[u128]) -> Vec<Entry> {
		mbid.iter()
			.map(|mbid| Entry {
				s: Source::from_u128(*mbid),
				q: 2,
				playlist: Vec::new(),
			})
			.collect()
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
		shard(work, "listen")
	}

	#[test]
	fn the_same_dump_keeps_what_a_previous_run_already_scanned() {
		let dir = dir("resume");
		let work = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();
		let partial = partial(&work);

		let again = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();

		assert_eq!(again, work);
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_throws_the_previous_scan_away() {
		let dir = dir("stale");
		let work = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();
		let partial = partial(&work);

		let _ = open(&dir, "20260809-000003", &declared(&[1, 2]));

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_work_directory_left_by_an_older_version_is_thrown_away() {
		let dir = dir("unstamped");
		let work = dir.join(DIR);
		let partial = partial(&work);

		let _ = open(&dir, "20260712-000004", &declared(&[1, 2]));

		assert!(!partial.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_new_declared_recording_throws_away_everything_keyed_by_recording_id() {
		let dir = dir("declared");
		let work = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();
		let partial = partial(&work);
		let seed = shard(&work, seed::NAME);
		let artist = shard(&work, artist::NAME);
		let listen = shard(&dir, USER_LISTEN);

		let _ = open(&dir, "20260712-000004", &declared(&[1, 2, 3]));

		assert!(partial.exists());
		assert!(!seed.exists());
		assert!(!artist.exists());
		assert!(!listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_same_declaration_in_another_order_is_the_same_recording_id() {
		let dir = dir("order");
		let work = open(&dir, "20260712-000004", &declared(&[2, 1])).unwrap_or_default();
		let seed = shard(&work, seed::NAME);
		let listen = shard(&dir, USER_LISTEN);

		let _ = open(&dir, "20260712-000004", &declared(&[1, 2, 2]));

		assert!(seed.exists());
		assert!(listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn another_dump_throws_away_the_published_user_listen() {
		let dir = dir("published");
		let _ = open(&dir, "20260712-000004", &declared(&[1, 2]));
		let listen = shard(&dir, USER_LISTEN);

		let _ = open(&dir, "20260809-000003", &declared(&[1, 2]));

		assert!(!listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_released_build_cannot_vouch_for_the_published_user_listen() {
		let dir = dir("released");
		let work = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();
		let listen = shard(&dir, USER_LISTEN);
		release(&work);

		let _ = open(&dir, "20260712-000004", &declared(&[1, 2]));

		assert!(!listen.exists());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn releasing_leaves_nothing_behind() {
		let dir = dir("release");
		let work = open(&dir, "20260712-000004", &declared(&[1, 2])).unwrap_or_default();
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

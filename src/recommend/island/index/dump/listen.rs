use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F, R, Y};
use hmerr::{GenericError, ge, ioe};
use indicatif::ProgressBar;

use super::{
	super::{keep, progress},
	board, rsync, space, stamp,
};

const MODULE: &str = "listenbrainz/fullexport";
pub(super) const PREFIX: &str = "listenbrainz-dump-";
const SUFFIX: &str = "-full";
const EXT: &str = ".tar";
const STAMP: &str = "TIMESTAMP";
const LISTEN: &str = "listen";
const DECLINED: &str = "declined-full";

pub(crate) struct Listen {
	pub dir: PathBuf,
	pub name: String,
}

pub(super) struct Offer {
	pub reason: &'static str,
	pub enter_is: bool,
}

pub(super) fn find(root: &Path) -> hmerr::Result<Option<Listen>> {
	let dir = root.join(LISTEN);

	if !holds_parquet(&dir)? {
		return Ok(None);
	}

	Ok(Some(Listen {
		name: name_of(&dir),
		dir,
	}))
}

fn name_of(dir: &Path) -> String {
	timestamp(dir).unwrap_or_else(|| {
		dir.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default()
	})
}

fn timestamp(dir: &Path) -> Option<String> {
	fs::read_to_string(dir.join(STAMP))
		.ok()
		.map(|stamp| stamp.trim().to_string())
}

fn holds_parquet(dir: &Path) -> hmerr::Result<bool> {
	if !dir.is_dir() {
		return Ok(false);
	}

	for entry in fs::read_dir(dir).map_err(|e| ioe!(dir.to_string_lossy(), e))? {
		let entry = entry.map_err(|e| ioe!(dir.to_string_lossy(), e))?;
		if entry.path().extension().is_some_and(|ext| ext == "parquet") {
			return Ok(true);
		}
	}

	Ok(false)
}

pub(super) fn fetch(root: &Path, offer: &Offer) -> hmerr::Result<Option<Listen>> {
	let dump =
		newest()?.ok_or_else(|| ge!(format!("{R}nothing published under {B}{MODULE}{D}")))?;

	fetch_named(root, &dump, offer)
}

pub(super) fn fetch_named(root: &Path, dump: &str, offer: &Offer) -> hmerr::Result<Option<Listen>> {
	let url = format!("{host}/{MODULE}/{dump}/", host = rsync::HOST);
	let archive = rsync::biggest(&url, EXT)?;
	let tar = root.join(&archive.name);

	progress::say(format!(
		"\n{F}listen dump {B}{dump}{D}{F}: {B}{Y}{size}{D}{F}, {B}{Y}+{size}{D}{F} unpacked, \
		deleted once index built{D}",
		size = progress::bytes(archive.size)
	));

	progress::say(format!("{F}{reason}{D}", reason = offer.reason));

	if !progress::ask("download", offer.enter_is)? {
		return Ok(None);
	}

	space::require(root, space::unpacking(&tar, archive.size))?;

	let board = board::listen(archive.size)?;

	board.run(board::DOWNLOAD, |bar| {
		rsync::pull(&format!("{url}{name}", name = archive.name), &tar, bar)
	})?;
	board.run(board::VERIFY, |_| {
		rsync::checked(&url, root, &rsync::checksum(&archive.name))
	})?;

	let dir = board.run(board::UNPACK, |bar| unpack(&tar, root, bar))?;
	keep::discard(&tar)?;

	Ok(Some(Listen {
		name: name_of(&dir),
		dir,
	}))
}

pub(super) fn newer_than(baseline: &str) -> hmerr::Result<Option<String>> {
	let built = stamp::reach(baseline)?;

	Ok(newest()?.filter(|name| reaches_past(name, built)))
}

fn reaches_past(name: &str, built: u64) -> bool {
	reaches(name).is_some_and(|reach| reach > built)
}

pub(super) fn declined(root: &Path) -> Option<String> {
	fs::read_to_string(root.join(DECLINED))
		.ok()
		.map(|name| name.trim().to_string())
}

pub(super) fn decline(root: &Path, dump: &str) -> hmerr::Result<()> {
	let path = root.join(DECLINED);
	fs::write(&path, dump).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn newest() -> hmerr::Result<Option<String>> {
	let published = rsync::list(&format!("{host}/{MODULE}/", host = rsync::HOST))?;

	Ok(newest_of(published.into_iter().map(|entry| entry.name)))
}

fn newest_of(name: impl Iterator<Item = String>) -> Option<String> {
	name.filter_map(|name| Some((number(&name)?, name)))
		.max()
		.map(|(_, name)| name)
}

fn number(name: &str) -> Option<u32> {
	stamp::published(name, PREFIX, SUFFIX).map(|published| published.number)
}

fn reaches(name: &str) -> Option<u64> {
	stamp::published(name, PREFIX, SUFFIX).map(|published| published.reach)
}

pub(super) fn discard(listen: &Listen) -> hmerr::Result<()> {
	if !listen.dir.is_dir() {
		return Ok(());
	}

	if !keep::requested() {
		progress::say(format!(
			"{F}index built, releasing its {B}{Y}{size}{D}{F} dump{D}",
			size = progress::bytes(weight(&listen.dir))
		));
	}

	keep::discard(&listen.dir)
}

fn weight(dir: &Path) -> u64 {
	let Ok(read) = fs::read_dir(dir) else {
		return 0;
	};

	read.filter_map(Result::ok)
		.filter_map(|entry| entry.metadata().ok())
		.filter(std::fs::Metadata::is_file)
		.map(|meta| meta.len())
		.sum()
}

fn unpack(tar: &Path, root: &Path, bar: &ProgressBar) -> hmerr::Result<PathBuf> {
	let file = fs::File::open(tar).map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	tar::Archive::new(bar.wrap_read(file))
		.unpack(root)
		.map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	let dir = root.join(LISTEN);
	let inner = fs::read_dir(root)
		.map_err(|e| ioe!(root.to_string_lossy(), e))?
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_dir() && *path != dir)
		.filter_map(|path| Some((timestamp(&path)?, path)))
		.max()
		.map(|(_, path)| path)
		.ok_or_else(|| ge!(format!("{R}the listen archive held no dump directory{D}")))?;

	if dir.exists() {
		fs::remove_dir_all(&dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;
	}
	fs::rename(&inner, &dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(dir)
}

pub(super) fn refused() -> GenericError {
	ge!(
		format!("{R}cancelled{D}"),
		h: "the index is built from the dump, no dump means nothing to recommend from"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";
	const BASELINE: &str = "listenbrainz-dump-2593-20260712-000004-full";

	fn scratch(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_dump_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn listen(dir: PathBuf) -> Listen {
		Listen {
			name: "test".to_string(),
			dir,
		}
	}

	fn repairs(name: &str, baseline: &str) -> bool {
		reaches_past(name, stamp::reach(baseline).unwrap_or_default())
	}

	fn published(name: &[&str]) -> Option<String> {
		newest_of(name.iter().map(|name| (*name).to_string()))
	}

	#[test]
	fn the_dump_number_is_read_out_of_the_published_name() {
		assert_eq!(
			number("listenbrainz-dump-2593-20260712-000004-full"),
			Some(2593)
		);
		assert_eq!(
			number("listenbrainz-dump-2593-20260712-000004-incremental"),
			None
		);
		assert_eq!(number("LATEST"), None);
	}

	#[test]
	fn the_newest_published_dump_is_the_highest_numbered_one() {
		assert_eq!(
			published(&[
				"listenbrainz-dump-2592-20260705-000003-full",
				"listenbrainz-dump-2593-20260712-000004-full",
			]),
			Some("listenbrainz-dump-2593-20260712-000004-full".to_string())
		);
	}

	#[test]
	fn a_wider_dump_number_is_still_the_newer_one() {
		assert_eq!(
			published(&[
				"listenbrainz-dump-1000-20340101-000001-full",
				"listenbrainz-dump-999-20330101-000001-full",
			]),
			Some("listenbrainz-dump-1000-20340101-000001-full".to_string())
		);
	}

	#[test]
	fn a_full_dump_published_past_the_baseline_is_the_one_that_repairs_a_gap() {
		assert!(
			repairs("listenbrainz-dump-2600-20260901-000003-full", BUILT),
			"what the index absorbed its way to is no reason to leave a hole unrepaired"
		);
	}

	#[test]
	fn the_dump_the_index_was_already_built_from_repairs_nothing() {
		assert!(!repairs(BASELINE, BUILT));
	}

	#[test]
	fn a_full_dump_older_than_the_baseline_repairs_nothing() {
		assert!(!repairs(
			"listenbrainz-dump-2592-20260705-000003-full",
			BUILT
		));
	}

	#[test]
	fn nothing_published_is_nothing_to_fetch() {
		assert_eq!(published(&["LATEST", "index.html"]), None);
	}

	#[test]
	fn a_directory_without_parquet_is_not_a_dump() {
		let root = scratch("empty");
		let _ = fs::create_dir_all(root.join(LISTEN));

		assert!(find(&root).unwrap_or_default().is_none());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn a_directory_holding_parquet_is_a_dump() {
		let root = scratch("holding");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"");

		assert!(find(&root).unwrap_or_default().is_some());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn the_dump_is_named_after_its_timestamp() {
		let root = scratch("stamp");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"");
		let _ = fs::write(dir.join(STAMP), b"  20260712-000004\n");

		assert_eq!(
			find(&root).unwrap_or_default().map(|dump| dump.name),
			Some("20260712-000004".to_string())
		);
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn discarding_removes_the_dump() {
		let root = scratch("discard");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"payload");

		let _ = discard(&listen(dir.clone()));

		assert!(!dir.exists());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn discarding_a_linked_dump_leaves_what_it_points_at_alone() {
		let root = scratch("linked");
		let dir = root.join(LISTEN);
		let elsewhere = root.join("elsewhere.parquet");
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(&elsewhere, b"payload");
		let _ = std::os::unix::fs::symlink(&elsewhere, dir.join("0.parquet"));

		let _ = discard(&listen(dir.clone()));

		assert!(!dir.exists());
		assert!(elsewhere.exists(), "the link target must survive");
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn discarding_what_is_already_gone_is_not_an_error() {
		let root = scratch("gone");

		assert!(discard(&listen(root.join(LISTEN))).is_ok());
		let _ = fs::remove_dir_all(&root);
	}
}

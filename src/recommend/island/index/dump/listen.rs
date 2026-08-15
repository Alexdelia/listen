use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, F, R, Y};
use hmerr::{GenericError, ge, ioe};

use super::{
	super::{keep, progress},
	rsync, space,
};

const MODULE: &str = "listenbrainz/fullexport";
const PREFIX: &str = "listenbrainz-dump-";
const SUFFIX: &str = "-full";
const EXT: &str = ".tar";
const STAMP: &str = "TIMESTAMP";
const LISTEN: &str = "listen";

pub(crate) struct Listen {
	pub dir: PathBuf,
	pub name: String,
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
	stamp(dir).unwrap_or_else(|| {
		dir.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default()
	})
}

fn stamp(dir: &Path) -> Option<String> {
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

pub(super) fn fetch(root: &Path) -> hmerr::Result<Listen> {
	let dump = newest()?;
	let url = format!("{host}/{MODULE}/{dump}/", host = rsync::HOST);
	let archive = rsync::biggest(&url, EXT)?;
	let tar = root.join(&archive.name);

	space::require(root, space::unpacking(&tar, archive.size))?;

	println!(
		"\n{F}the listen dump {B}{dump}{D}{F} is {B}{Y}{size}{D}{F}, and needs {B}{Y}{size}{D}{F} more once unpacked.{D}\n\
		{F}it is only ever read to build the index, and deleted as soon as the index is built.{D}",
		size = progress::bytes(archive.size)
	);

	if !ux::ask_yn("download it", false).map_err(|e| ioe!("stdin", e))? {
		return Err(refused().into());
	}

	let checksum = format!(
		"{name}{ext}",
		name = archive.name,
		ext = rsync::CHECKSUM_EXT
	);

	rsync::pull(
		&format!("{url}{name}", name = archive.name),
		&tar,
		archive.size,
	)?;
	rsync::small(&format!("{url}{checksum}"), &root.join(&checksum))?;
	rsync::verify(root, &checksum)?;
	rsync::forget(root, &[&checksum])?;

	let dir = unpack(&tar, root, archive.size)?;
	keep::discard(&tar)?;

	Ok(Listen {
		name: name_of(&dir),
		dir,
	})
}

fn newest() -> hmerr::Result<String> {
	let published = rsync::list(&format!("{host}/{MODULE}/", host = rsync::HOST))?;

	newest_of(published.into_iter().map(|entry| entry.name))
		.ok_or_else(|| ge!(format!("{R}nothing published under {B}{MODULE}{D}")).into())
}

fn newest_of(name: impl Iterator<Item = String>) -> Option<String> {
	name.filter_map(|name| Some((number(&name)?, name)))
		.max()
		.map(|(_, name)| name)
}

fn number(name: &str) -> Option<u32> {
	name.strip_prefix(PREFIX)?
		.strip_suffix(SUFFIX)?
		.split('-')
		.next()?
		.parse()
		.ok()
}

pub(super) fn discard(listen: &Listen) -> hmerr::Result<()> {
	if !listen.dir.is_dir() {
		return Ok(());
	}

	if !keep::requested() {
		println!(
			"{F}the index is built, releasing the {B}{Y}{size}{D}{F} dump it came from{D}",
			size = progress::bytes(weight(&listen.dir))
		);
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

fn unpack(tar: &Path, root: &Path, size: u64) -> hmerr::Result<PathBuf> {
	let bar = progress::byte_bar(size, "unpack")?;
	let file = fs::File::open(tar).map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	tar::Archive::new(bar.wrap_read(file))
		.unpack(root)
		.map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	bar.finish();

	let dir = root.join(LISTEN);
	let inner = fs::read_dir(root)
		.map_err(|e| ioe!(root.to_string_lossy(), e))?
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_dir() && *path != dir)
		.filter_map(|path| Some((stamp(&path)?, path)))
		.max()
		.map(|(_, path)| path)
		.ok_or_else(|| ge!(format!("{R}the listen archive held no dump directory{D}")))?;

	if dir.exists() {
		fs::remove_dir_all(&dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;
	}
	fs::rename(&inner, &dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(dir)
}

fn refused() -> GenericError {
	ge!(
		format!("{R}cancelled{D}"),
		h: "the index is built from the dump, so there is nothing to recommend from without it"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

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

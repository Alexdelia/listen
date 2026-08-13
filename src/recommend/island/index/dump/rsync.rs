use std::{fs, path::Path, process::Command};

use ansi::abbrev::{B, D, F, R};
use hmerr::{GenericError, ge, ioe};

use super::super::progress;

pub(super) const PROGRAM: &str = "rsync";
pub(super) const HOST: &str = "rsync://data.metabrainz.org/musicbrainz";

pub(super) const CHECKSUM_EXT: &str = ".sha256";

const SHA256SUM: &str = "sha256sum";
const MARKER: &str = "LATEST";

pub(super) struct Entry {
	pub name: String,
	pub size: u64,
}

pub(super) fn list(url: &str) -> hmerr::Result<Vec<Entry>> {
	let out = Command::new(PROGRAM)
		.args(["--list-only", url])
		.output()
		.map_err(|e| missing_rsync(&e.to_string()))?;

	if !out.status.success() {
		return Err(ge!(format!(
			"{R}{B}{PROGRAM}{D}{R} could not list {B}{url}{D}\n{}{D}",
			String::from_utf8_lossy(&out.stderr).trim()
		))
		.into());
	}

	Ok(String::from_utf8_lossy(&out.stdout)
		.lines()
		.filter_map(parse)
		.collect())
}

fn parse(line: &str) -> Option<Entry> {
	let mut field = line.split_whitespace();
	let _mode = field.next()?;
	let size = field.next()?.replace(',', "").parse().ok()?;
	let _date = field.next()?;
	let _time = field.next()?;
	let name = field.next()?;

	if name == "." || name == ".." {
		return None;
	}

	Some(Entry {
		name: name.to_string(),
		size,
	})
}

pub(super) fn small(url: &str, into: &Path) -> hmerr::Result<()> {
	prepare(into)?;

	let status = Command::new(PROGRAM)
		.args(["--quiet", url, &into.to_string_lossy()])
		.status()
		.map_err(|e| missing_rsync(&e.to_string()))?;

	if !status.success() {
		return Err(ge!(format!("{R}{B}{PROGRAM}{D}{R} could not fetch {B}{url}{D}")).into());
	}

	Ok(())
}

pub(super) fn pull(url: &str, into: &Path, total: u64) -> hmerr::Result<()> {
	prepare(into)?;

	progress::rsync(
		PROGRAM,
		&[
			"--archive",
			"--partial",
			"--info=progress2",
			"--no-inc-recursive",
			url,
			&into.to_string_lossy(),
		],
		total,
	)
}

fn prepare(into: &Path) -> hmerr::Result<()> {
	let Some(dir) = into
		.parent()
		.filter(|parent| !parent.as_os_str().is_empty())
	else {
		return Ok(());
	};

	fs::create_dir_all(dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(())
}

pub(super) fn verify(dir: &Path, sums: &str) -> hmerr::Result<()> {
	if !dir.join(sums).exists() {
		println!("{F}no {B}{sums}{D}{F} alongside the archive, skipping verification{D}");
		return Ok(());
	}

	println!("{F}verifying against {B}{sums}{D}");

	let status = Command::new(SHA256SUM)
		.args(["--check", "--ignore-missing", sums])
		.current_dir(dir)
		.status()
		.map_err(|e| ge!(format!("{R}failed to execute {B}{SHA256SUM}{D}\n{e}")))?;

	if !status.success() {
		return Err(corrupt(dir).into());
	}

	Ok(())
}

pub(super) fn latest_marker(module: &str, into: &Path) -> hmerr::Result<String> {
	let marker = into.join(MARKER);
	small(&format!("{HOST}/{module}/{MARKER}"), &marker)?;

	let name = fs::read_to_string(&marker).map_err(|e| ioe!(marker.to_string_lossy(), e))?;
	forget(into, &[MARKER])?;

	Ok(name.trim().to_string())
}

pub(super) fn forget(dir: &Path, name: &[&str]) -> hmerr::Result<()> {
	for name in name {
		let path = dir.join(name);

		if path.exists() {
			fs::remove_file(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
		}
	}

	Ok(())
}

pub(super) fn newest_dir(module: &str, prefix: &str, suffix: &str) -> hmerr::Result<String> {
	list(&format!("{HOST}/{module}/"))?
		.into_iter()
		.map(|entry| entry.name)
		.filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
		.max()
		.ok_or_else(|| ge!(format!("{R}nothing published under {B}{module}{D}")).into())
}

pub(super) fn biggest(url: &str, ext: &str) -> hmerr::Result<Entry> {
	list(url)?
		.into_iter()
		.filter(|entry| entry.name.ends_with(ext))
		.max_by_key(|entry| entry.size)
		.ok_or_else(|| ge!(format!("{R}no {B}{ext}{D}{R} inside {B}{url}{D}")).into())
}

fn missing_rsync(reason: &str) -> GenericError {
	ge!(
		format!("{R}failed to execute {B}{PROGRAM}{D}\n{reason}"),
		h: format!("{B}{PROGRAM}{D} downloads the dumps, it comes with the nix dev shell")
	)
}

fn corrupt(dir: &Path) -> GenericError {
	ge!(
		format!("{R}a downloaded archive does not match its published checksum{D}"),
		h: format!("delete it under {B}{}{D} and run again", dir.display())
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_listing_line_yields_a_name_and_a_size() {
		let entry = parse("-rw-r--r-- 205,073,162,240 2026/07/16 19:30:43 dump.tar");

		assert_eq!(
			entry.as_ref().map(|entry| entry.name.as_str()),
			Some("dump.tar")
		);
		assert_eq!(entry.map(|entry| entry.size), Some(205_073_162_240));
	}

	#[test]
	fn a_directory_line_yields_its_name() {
		let entry = parse("drwxr-xr-x 4,096 2026/07/16 19:59:37 listenbrainz-dump-2593-full");

		assert_eq!(
			entry.map(|entry| entry.name),
			Some("listenbrainz-dump-2593-full".to_string())
		);
	}

	#[test]
	fn the_current_and_parent_directory_are_skipped() {
		assert!(parse("drwxr-xr-x 4,096 2026/07/16 19:49:39 .").is_none());
		assert!(parse("drwxr-xr-x 4,096 2026/07/16 19:49:39 ..").is_none());
	}

	#[test]
	fn a_short_line_is_not_a_listing() {
		assert!(parse("").is_none());
		assert!(parse("receiving file list ... done").is_none());
	}
}

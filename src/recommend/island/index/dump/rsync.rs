use std::{fs, path::Path, process::Command};

use ansi::abbrev::{B, D, F, R};
use hmerr::{GenericError, ge, ioe};

use super::super::{keep, progress};

pub(super) const PROGRAM: &str = "rsync";
pub(super) const HOST: &str = "rsync://data.metabrainz.org/musicbrainz";

pub(super) const CHECKSUM_EXT: &str = ".sha256";

const SHA256SUM: &str = "sha256sum";
const DIGEST_LEN: usize = 64;
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
	let path = dir.join(sums);

	if !path.exists() {
		progress::say(format!(
			"{F}no {B}{sums}{D}{F} alongside the archive, verification skipped{D}"
		));
		return Ok(());
	}

	progress::say(format!("{F}verifying against {B}{sums}{D}"));

	let published = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	let matches = match lone_digest(&published) {
		Some(digest) => archive_digest(dir, digest_owner(sums))?.eq_ignore_ascii_case(digest),
		None => checked_list(dir, sums)?,
	};

	if !matches {
		return Err(corrupt(dir).into());
	}

	Ok(())
}

fn lone_digest(published: &str) -> Option<&str> {
	let digest = published.trim();

	(digest.len() == DIGEST_LEN && digest.chars().all(|c| c.is_ascii_hexdigit())).then_some(digest)
}

fn digest_owner(sums: &str) -> &str {
	sums.strip_suffix(CHECKSUM_EXT).unwrap_or(sums)
}

fn archive_digest(dir: &Path, archive: &str) -> hmerr::Result<String> {
	let out = Command::new(SHA256SUM)
		.arg(archive)
		.current_dir(dir)
		.output()
		.map_err(|e| ge!(format!("{R}failed to execute {B}{SHA256SUM}{D}\n{e}")))?;

	if !out.status.success() {
		return Err(ge!(format!(
			"{R}{B}{SHA256SUM}{D}{R} could not read {B}{archive}{D}\n{}{D}",
			String::from_utf8_lossy(&out.stderr).trim()
		))
		.into());
	}

	Ok(String::from_utf8_lossy(&out.stdout)
		.split_whitespace()
		.next()
		.unwrap_or_default()
		.to_string())
}

fn checked_list(dir: &Path, sums: &str) -> hmerr::Result<bool> {
	let out = Command::new(SHA256SUM)
		.args(["--check", "--ignore-missing", sums])
		.current_dir(dir)
		.output()
		.map_err(|e| ge!(format!("{R}failed to execute {B}{SHA256SUM}{D}\n{e}")))?;

	Ok(out.status.success())
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
		keep::discard(&dir.join(name))?;
	}

	Ok(())
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
		h: format!("{B}{PROGRAM}{D} downloads the dump, it comes with the nix dev shell")
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

	const PAYLOAD: &[u8] = b"payload";
	const PAYLOAD_DIGEST: &str = "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5";

	fn scratch(name: &str) -> std::path::PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_rsync_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn archive(dir: &Path, sums: &str, published: &str) {
		let _ = fs::write(dir.join("dump.tar"), PAYLOAD);
		let _ = fs::write(dir.join(sums), published);
	}

	#[test]
	fn a_published_file_holding_only_a_digest_is_a_lone_digest() {
		assert_eq!(lone_digest(PAYLOAD_DIGEST), Some(PAYLOAD_DIGEST));
		assert_eq!(
			lone_digest(&format!("{PAYLOAD_DIGEST}\n")),
			Some(PAYLOAD_DIGEST)
		);
		assert_eq!(lone_digest(&format!("{PAYLOAD_DIGEST}  dump.tar")), None);
		assert_eq!(lone_digest("not a digest"), None);
	}

	#[test]
	fn a_lone_digest_names_the_archive_it_covers() {
		assert_eq!(digest_owner("dump.tar.sha256"), "dump.tar");
		assert_eq!(digest_owner("SHA256SUMS"), "SHA256SUMS");
	}

	#[test]
	fn an_archive_matching_its_lone_digest_verifies() {
		let dir = scratch("lone");
		archive(&dir, "dump.tar.sha256", &format!("{PAYLOAD_DIGEST}\n"));

		assert!(verify(&dir, "dump.tar.sha256").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_archive_against_another_lone_digest_is_corrupt() {
		let dir = scratch("lone_corrupt");
		archive(&dir, "dump.tar.sha256", &"0".repeat(DIGEST_LEN));

		assert!(verify(&dir, "dump.tar.sha256").is_err());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_archive_listed_in_a_checksum_list_verifies() {
		let dir = scratch("list");
		archive(&dir, "SHA256SUMS", &format!("{PAYLOAD_DIGEST} *dump.tar\n"));

		assert!(verify(&dir, "SHA256SUMS").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_missing_checksum_skips_verification() {
		let dir = scratch("absent");

		assert!(verify(&dir, "SHA256SUMS").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}
}

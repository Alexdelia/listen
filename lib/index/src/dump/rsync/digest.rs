use std::{
	fs,
	path::Path,
	process::{Command, Output},
};

use console::style;
use miette::{IntoDiagnostic, WrapErr, miette};

use super::super::super::progress;

const CHECKSUM_EXT: &str = ".sha256";

const SHA256SUM: &str = "sha256sum";
const DIGEST_LEN: usize = 64;

pub(crate) fn checksum(name: &str) -> String {
	format!("{name}{CHECKSUM_EXT}")
}

pub(super) fn verify(dir: &Path, sums: &str) -> miette::Result<()> {
	let path = dir.join(sums);

	if !path.exists() {
		progress::say(format!(
			"{}{}{}",
			style("no ").dim(),
			style(sums).dim().bold(),
			style(" alongside the archive, verification skipped").dim()
		));
		return Ok(());
	}

	let published = fs::read_to_string(&path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	let matches = match lone_digest(&published) {
		Some(digest) => archive_digest(dir, digest_owner(sums))?.eq_ignore_ascii_case(digest),
		None => checked_list(dir, sums)?,
	};

	if !matches {
		return Err(corrupt(dir));
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

fn archive_digest(dir: &Path, archive: &str) -> miette::Result<String> {
	let out = summed(&[archive], dir)?;

	if !out.status.success() {
		return Err(miette!(
			"{SHA256SUM} could not read {archive}\n{}",
			String::from_utf8_lossy(&out.stderr).trim()
		));
	}

	Ok(String::from_utf8_lossy(&out.stdout)
		.split_whitespace()
		.next()
		.unwrap_or_default()
		.to_string())
}

fn checked_list(dir: &Path, sums: &str) -> miette::Result<bool> {
	Ok(summed(&["--check", "--ignore-missing", sums], dir)?
		.status
		.success())
}

fn summed(argument: &[&str], dir: &Path) -> miette::Result<Output> {
	Command::new(SHA256SUM)
		.args(argument)
		.current_dir(dir)
		.output()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {SHA256SUM}"))
}

fn corrupt(dir: &Path) -> miette::Report {
	miette!(
		help = format!("delete it under {} and run again", dir.display()),
		"a downloaded archive does not match its published checksum"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	const PAYLOAD: &[u8] = b"payload";
	const PAYLOAD_DIGEST: &str = "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5";

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
		let dir = crate::scratch::of("rsync", "lone");
		archive(&dir, "dump.tar.sha256", &format!("{PAYLOAD_DIGEST}\n"));

		assert!(verify(&dir, "dump.tar.sha256").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_archive_against_another_lone_digest_is_corrupt() {
		let dir = crate::scratch::of("rsync", "lone_corrupt");
		archive(&dir, "dump.tar.sha256", &"0".repeat(DIGEST_LEN));

		assert!(verify(&dir, "dump.tar.sha256").is_err());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_archive_listed_in_a_checksum_list_verifies() {
		let dir = crate::scratch::of("rsync", "list");
		archive(&dir, "SHA256SUMS", &format!("{PAYLOAD_DIGEST} *dump.tar\n"));

		assert!(verify(&dir, "SHA256SUMS").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_missing_checksum_skips_verification() {
		let dir = crate::scratch::of("rsync", "absent");

		assert!(verify(&dir, "SHA256SUMS").is_ok());
		let _ = fs::remove_dir_all(&dir);
	}
}

use std::{fs, path::Path};

use miette::miette;

use super::super::progress;

pub(super) fn unpacking(archive: &Path, size: u64) -> u64 {
	let alongside = size;

	size.saturating_add(alongside)
		.saturating_sub(resumed(archive))
}

pub(super) fn require(dir: &Path, need: u64) -> miette::Result<()> {
	let free = free(dir)?;

	if free >= need {
		return Ok(());
	}

	Err(short(dir, need, free))
}

fn resumed(archive: &Path) -> u64 {
	fs::metadata(archive).map_or(0, |meta| meta.len())
}

fn free(dir: &Path) -> miette::Result<u64> {
	let stat = rustix::fs::statvfs(dir).map_err(|e| {
		miette!(
			"could not measure the free space of {dir}\n{e}",
			dir = dir.display()
		)
	})?;

	Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

fn short(dir: &Path, need: u64, free: u64) -> miette::Report {
	miette!(
		help = "peak is archive plus what unpacks beside it, archive deleted right after unpack",
		"the dump needs {need} at peak, {dir} holds {free}",
		dir = dir.display(),
		free = progress::bytes(free),
		need = progress::bytes(need)
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_mounted_directory_reports_its_free_space() {
		assert!(free(&std::env::temp_dir()).unwrap_or_default() > 0);
	}

	#[test]
	fn an_absent_directory_cannot_be_measured() {
		assert!(free(Path::new("/nowhere/at/all")).is_err());
	}

	#[test]
	fn what_fits_is_not_refused() {
		assert!(require(&std::env::temp_dir(), 1).is_ok());
	}

	#[test]
	fn what_cannot_fit_is_refused() {
		assert!(require(&std::env::temp_dir(), u64::MAX).is_err());
	}

	#[test]
	fn an_untouched_dump_needs_the_archive_twice() {
		assert_eq!(unpacking(Path::new("/nowhere/absent.tar"), 191), 382);
	}

	#[test]
	fn a_resumed_download_needs_what_is_not_on_disk_yet() {
		let tar = std::env::temp_dir().join("declarative_listen_space_resumed.tar");
		let _ = fs::write(&tar, b"1234567890");

		assert_eq!(unpacking(&tar, 100), 190);
		let _ = fs::remove_file(&tar);
	}

	#[test]
	fn a_download_larger_than_the_peak_needs_nothing_more() {
		let tar = std::env::temp_dir().join("declarative_listen_space_whole.tar");
		let _ = fs::write(&tar, b"1234567890");

		assert_eq!(unpacking(&tar, 2), 0);
		let _ = fs::remove_file(&tar);
	}
}

use ansi::abbrev::{B, D, R};
use hmerr::ge;

use super::ran;

pub(crate) struct Entry {
	pub name: String,
	pub size: u64,
}

pub(crate) fn list(url: &str) -> hmerr::Result<Vec<Entry>> {
	listed(&["--list-only", url], url)
}

pub(crate) fn beneath(url: &str, pattern: &str) -> hmerr::Result<Vec<Entry>> {
	listed(
		&[
			"--list-only",
			"--recursive",
			"--include=*/",
			&format!("--include={pattern}"),
			"--exclude=*",
			url,
		],
		url,
	)
}

pub(crate) fn biggest(url: &str, ext: &str) -> hmerr::Result<Entry> {
	list(url)?
		.into_iter()
		.filter(|entry| entry.name.ends_with(ext))
		.max_by_key(|entry| entry.size)
		.ok_or_else(|| ge!(format!("{R}no {B}{ext}{D}{R} inside {B}{url}{D}")).into())
}

fn listed(argument: &[&str], url: &str) -> hmerr::Result<Vec<Entry>> {
	let out = ran(argument, "list", url)?;

	Ok(String::from_utf8_lossy(&out)
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

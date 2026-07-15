use std::{
	collections::HashMap,
	path::Path,
	process::Command,
	time::{SystemTime, UNIX_EPOCH},
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::declaration::Source;

const COMMIT_TIMESTAMP_FORMAT: &str = "--format=%ct";
const SOURCE_PREFIX: &str = "s: \"";
const SECOND_PER_DAY: u64 = 60 * 60 * 24;

pub(super) type Age = HashMap<Source, u64>;

pub(super) fn days_since_added(path: &Path) -> hmerr::Result<Age> {
	let output = Command::new("git")
		.arg("log")
		.arg("--reverse")
		.arg(COMMIT_TIMESTAMP_FORMAT)
		.arg("-p")
		.arg("--")
		.arg(path)
		.output()
		.map_err(|e| ioe!("git log", e))?;

	if !output.status.success() {
		return Err(ge!(format!(
			"{R}git log failed for {B}{path}{D}\n{err}",
			path = path.to_string_lossy(),
			err = String::from_utf8_lossy(&output.stderr),
		))
		.into());
	}

	Ok(parse_log(&String::from_utf8_lossy(&output.stdout), now()?))
}

fn now() -> hmerr::Result<u64> {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs())
		.map_err(|e| ge!(format!("{R}system clock is before the unix epoch\n{e}")).into())
}

fn parse_log(log: &str, now: u64) -> Age {
	let mut age = Age::new();
	let mut days = 0;
	let mut removed: Vec<(String, Source)> = Vec::new();

	for line in log.lines() {
		if let Some(timestamp) = commit_timestamp(line) {
			days = now.saturating_sub(timestamp) / SECOND_PER_DAY;
			removed.clear();
		} else if let Some(deleted) = line.strip_prefix('-') {
			if let Some(entry) = split_source(deleted) {
				removed.push(entry);
			}
		} else if let Some(added) = line.strip_prefix('+')
			&& let Some((rest, mbid)) = split_source(added)
		{
			let inherited = replaced(&mut removed, &rest)
				.and_then(|old| age.get(&old).copied())
				.unwrap_or(days);

			age.entry(mbid).or_insert(inherited);
		}
	}

	age
}

fn replaced(removed: &mut Vec<(String, Source)>, rest: &str) -> Option<Source> {
	let i = removed.iter().position(|(r, _)| r == rest)?;

	Some(removed.remove(i).1)
}

fn commit_timestamp(line: &str) -> Option<u64> {
	if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) {
		return None;
	}

	line.parse().ok()
}

fn split_source(content: &str) -> Option<(String, Source)> {
	if content.trim_start().starts_with("//") {
		return None;
	}

	let start = content.find(SOURCE_PREFIX)? + SOURCE_PREFIX.len();
	let end = start + content[start..].find('"')?;

	Some((
		[&content[..start], &content[end..]].concat(),
		content[start..end].parse().ok()?,
	))
}

#[cfg(test)]
mod tests {
	use const_format::formatcp;

	use super::*;

	const AAAA: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
	const BBBB: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
	const CCCC: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";
	const OLD: &str = "dddddddd-dddd-dddd-dddd-dddddddddddd";
	const SWAP: &str = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
	const BORN: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";

	fn id(mbid: &str) -> Source {
		mbid.parse().unwrap_or_default()
	}

	const LOG: &str = formatcp!(
		"\
100000
diff --git a/listen.ron b/listen.ron
--- a/listen.ron
+++ b/listen.ron
@@ -0,0 +1,3 @@
+[
+\t(s: \"{AAAA}\", q: 4, playlist: []),
+\t// (s: \"{CCCC}\", q: 2, playlist: []),
300000
diff --git a/listen.ron b/listen.ron
--- a/listen.ron
+++ b/listen.ron
@@ -1,3 +1,4 @@
-\t(s: \"{AAAA}\", q: 4, playlist: []),
+\t(s: \"{AAAA}\", q: 2, playlist: []),
+\t(s: \"{BBBB}\", q: 1, playlist: []),
+\t(s: \"{CCCC}\", q: 2, playlist: []),"
	);

	#[test]
	fn extract_source() {
		assert_eq!(
			split_source(formatcp!("\t(s: \"{AAAA}\", q: 4")),
			Some(("\t(s: \"\", q: 4".to_string(), id(AAAA)))
		);
	}

	#[test]
	fn skip_comment() {
		assert_eq!(split_source(formatcp!("// (s: \"{CCCC}\", q: 2")), None);
	}

	#[test]
	fn skip_invalid_mbid() {
		assert_eq!(split_source("\t(s: \"not-an-mbid\", q: 4"), None);
	}

	#[test]
	fn no_source() {
		assert_eq!(split_source("["), None);
	}

	#[test]
	fn edit_keeps_first_added_date() {
		let age = parse_log(LOG, 300_000 + 5 * SECOND_PER_DAY);

		assert_eq!(age.get(&id(AAAA)), Some(&7));
		assert_eq!(age.get(&id(BBBB)), Some(&5));
	}

	#[test]
	fn uncommenting_counts_as_added() {
		let age = parse_log(LOG, 300_000 + 5 * SECOND_PER_DAY);

		assert_eq!(age.get(&id(CCCC)), Some(&5));
	}

	const SWAP_LOG: &str = formatcp!(
		"\
100000
diff --git a/listen.ron b/listen.ron
--- a/listen.ron
+++ b/listen.ron
@@ -0,0 +1,1 @@
+\t(s: \"{OLD}\", q: 2, playlist: [\"aggressive\"]),
300000
diff --git a/listen.ron b/listen.ron
--- a/listen.ron
+++ b/listen.ron
@@ -1,1 +1,2 @@
-\t(s: \"{OLD}\", q: 2, playlist: [\"aggressive\"]),
+\t(s: \"{SWAP}\", q: 2, playlist: [\"aggressive\"]),
+\t(s: \"{BORN}\", q: 2, playlist: []),"
	);

	#[test]
	fn source_swap_inherits_first_added_date() {
		let age = parse_log(SWAP_LOG, 300_000 + 5 * SECOND_PER_DAY);

		assert_eq!(age.get(&id(SWAP)), Some(&7));
		assert_eq!(age.get(&id(BORN)), Some(&5));
	}
}

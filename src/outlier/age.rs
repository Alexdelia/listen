use std::{
	collections::HashMap,
	path::Path,
	process::Command,
	time::{SystemTime, UNIX_EPOCH},
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::entry::Source;

const COMMITTER_TIME: &str = "committer-time ";
const SOURCE_PREFIX: &str = "s: \"";
const SHA_LEN: usize = 40;
const SECOND_PER_DAY: u64 = 60 * 60 * 24;

pub(super) type Age = HashMap<Source, u64>;

pub(super) fn days_since_added(path: &Path) -> hmerr::Result<Age> {
	let output = Command::new("git")
		.arg("blame")
		.arg("--line-porcelain")
		.arg("--")
		.arg(path)
		.output()
		.map_err(|e| ioe!("git blame", e))?;

	if !output.status.success() {
		return Err(ge!(format!(
			"{R}git blame failed for {B}{path}{D}\n{err}",
			path = path.to_string_lossy(),
			err = String::from_utf8_lossy(&output.stderr),
		))
		.into());
	}

	Ok(parse_blame(
		&String::from_utf8_lossy(&output.stdout),
		now()?,
	))
}

fn now() -> hmerr::Result<u64> {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs())
		.map_err(|e| ge!(format!("{R}system clock is before the unix epoch\n{e}")).into())
}

fn parse_blame(blame: &str, now: u64) -> Age {
	let mut age = Age::new();
	let mut committed = HashMap::new();
	let mut sha = "";

	for line in blame.lines() {
		if let Some(timestamp) = line.strip_prefix(COMMITTER_TIME) {
			if let Ok(timestamp) = timestamp.trim().parse::<u64>() {
				committed.insert(sha, timestamp);
			}
		} else if let Some(content) = line.strip_prefix('\t') {
			if let Some(mbid) = source(content) {
				let days = committed.get(sha).map_or(0, |timestamp| {
					now.saturating_sub(*timestamp) / SECOND_PER_DAY
				});
				age.insert(mbid, days);
			}
		} else if let Some(header) = header_sha(line) {
			sha = header;
		}
	}

	age
}

fn header_sha(line: &str) -> Option<&str> {
	let sha = line.split(' ').next()?;

	(sha.len() == SHA_LEN && sha.bytes().all(|b| b.is_ascii_hexdigit())).then_some(sha)
}

fn source(content: &str) -> Option<Source> {
	if content.trim_start().starts_with("//") {
		return None;
	}

	let start = content.find(SOURCE_PREFIX)? + SOURCE_PREFIX.len();
	let rest = &content[start..];

	Some(rest[..rest.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	const BLAME: &str = "\
0000000000000000000000000000000000000000 1 1 1
author alex
committer-time 200000
	[
1111111111111111111111111111111111111111 2 2 1
author alex
committer-time 100000
	(s: \"aaaa\", q: 4, playlist: []),
1111111111111111111111111111111111111111 3 3
	(s: \"bbbb\", q: 1, playlist: []),
2222222222222222222222222222222222222222 4 4 1
author alex
committer-time 300000
	// (s: \"cccc\", q: 2, playlist: []),";

	#[test]
	fn extract_source() {
		assert_eq!(source("\t(s: \"aaaa\", q: 4").as_deref(), Some("aaaa"));
		assert_eq!(source("(s: \"aaaa\", q: 4"), Some("aaaa".to_string()));
	}

	#[test]
	fn skip_comment() {
		assert_eq!(source("// (s: \"cccc\", q: 2"), None);
	}

	#[test]
	fn no_source() {
		assert_eq!(source("["), None);
	}

	#[test]
	fn blame_reuses_committer_time() {
		let age = parse_blame(BLAME, 100000 + 5 * SECOND_PER_DAY);

		assert_eq!(age.get("aaaa"), Some(&5));
		assert_eq!(age.get("bbbb"), Some(&5));
		assert_eq!(age.get("cccc"), None);
	}
}

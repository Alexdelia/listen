pub mod sync;

use std::{
	collections::HashSet,
	path::{Path, PathBuf},
};

use hmerr::se;

use crate::entry::{Q, Source};

pub const OUTPUT_DIR: &str = "./output/playlist";

pub const PREFIX: &str = "+q";

pub const EXT: &str = "m3u";

pub fn q_stem(q: u8) -> String {
	format!("{PREFIX}{q}")
}

pub fn q_path(q: u8) -> PathBuf {
	PathBuf::from(OUTPUT_DIR)
		.join(q_stem(q))
		.with_extension(EXT)
}

pub fn playlist_path(playlist: &str) -> PathBuf {
	PathBuf::from(OUTPUT_DIR).join(playlist).with_extension(EXT)
}

pub fn parse_q(name: &str) -> hmerr::Result<Q> {
	let q = name.trim_start_matches(PREFIX);
	Ok(q.parse().map_err(|e| {
		se!(
			"failed to parse q from {name}",
			"u8",
			q,
			s: e
		)
	})?)
}

pub fn parse_content(content: &str) -> HashSet<Source> {
	let mut set = HashSet::<Source>::new();

	for line in content.lines() {
		if line.starts_with('#') {
			continue;
		}

		set.insert(
			Path::new(line)
				.file_stem()
				.unwrap_or_else(|| panic!("failed to get file stem from {line}"))
				.to_string_lossy()
				.to_string(),
		);
	}

	set
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_q_stem() {
		for i in 0..10 {
			assert_eq!(q_stem(i), format!("+q{i}"));
		}
	}

	#[test]
	fn test_q_path() {
		for i in 0..10 {
			assert_eq!(
				q_path(i),
				PathBuf::from(format!("./output/playlist/+q{i}.m3u"))
			);
		}
	}

	#[test]
	fn test_parse_q() {
		for i in 0..=255 as Q {
			let res = parse_q(&format!("+q{i}"));
			match res {
				Ok(q) => assert_eq!(q, i),
				Err(e) => panic!("{:?}", e),
			}
		}
	}
}

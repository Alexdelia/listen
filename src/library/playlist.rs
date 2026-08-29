use std::{
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
};

use hmerr::{ioe, se};

use crate::declaration::{Q, Source};

use super::file;

pub(crate) const DIR: &str = "./output/playlist";

pub(crate) const PREFIX: &str = "+q";

pub(crate) const EXT: &str = "m3u";

pub(crate) fn q_stem(q: Q) -> String {
	format!("{PREFIX}{q}")
}

pub(crate) fn q_path(q: Q) -> PathBuf {
	PathBuf::from(DIR).join(q_stem(q)).with_extension(EXT)
}

pub(crate) fn path(playlist: &str) -> PathBuf {
	PathBuf::from(DIR).join(playlist).with_extension(EXT)
}

pub(crate) fn parse_q(name: &str) -> hmerr::Result<Q> {
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

pub(crate) fn header(content: &str) -> String {
	content
		.lines()
		.take_while(|line| line.starts_with('#'))
		.collect::<Vec<_>>()
		.join("\n")
}

pub(crate) fn parse_content(content: &str) -> HashSet<Source> {
	let mut set = HashSet::<Source>::new();

	for line in content.lines() {
		if line.starts_with('#') {
			continue;
		}

		let Some(source) = Path::new(line)
			.file_stem()
			.and_then(|stem| stem.to_str())
			.and_then(|stem| stem.parse().ok())
		else {
			continue;
		};

		set.insert(source);
	}

	set
}

#[derive(Default)]
pub(crate) struct Existing {
	pub q: HashMap<Q, HashSet<Source>>,
	pub playlist: HashMap<String, HashSet<Source>>,
}

pub(crate) fn existing() -> hmerr::Result<Existing> {
	let mut ret = Existing::default();

	for found in file::with_extension(DIR, EXT)? {
		let list = parse_content(
			&fs::read_to_string(&found.path).map_err(|e| ioe!(found.path.to_string_lossy(), e))?,
		);

		if found.stem.starts_with(PREFIX) {
			ret.q.insert(parse_q(&found.stem)?, list);
		} else {
			ret.playlist.insert(found.stem, list);
		}
	}

	Ok(ret)
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
	fn the_header_is_what_a_playlist_says_before_its_first_entry() {
		assert_eq!(
			header("#EXTM3U\n#the ones that carry a run\n/a.mp3\n/b.mp3"),
			"#EXTM3U\n#the ones that carry a run"
		);
	}

	#[test]
	fn a_playlist_saying_nothing_before_its_first_entry_has_no_header() {
		assert_eq!(header("/a.mp3\n#a word after the fact\n/b.mp3"), "");
		assert_eq!(header(""), "");
	}

	#[test]
	fn test_parse_q() {
		for i in 0..=255 as Q {
			let res = parse_q(&format!("+q{i}"));
			match res {
				Ok(q) => assert_eq!(q, i),
				Err(e) => panic!("{e:?}"),
			}
		}
	}
}

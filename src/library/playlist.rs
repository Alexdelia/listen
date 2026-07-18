use std::{
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
};

use hmerr::{ioe, se};

use crate::declaration::{Q, Source};

pub const DIR: &str = "./output/playlist";

pub const PREFIX: &str = "+q";

pub const EXT: &str = "m3u";

pub fn q_stem(q: Q) -> String {
	format!("{PREFIX}{q}")
}

pub fn q_path(q: Q) -> PathBuf {
	PathBuf::from(DIR).join(q_stem(q)).with_extension(EXT)
}

pub fn path(playlist: &str) -> PathBuf {
	PathBuf::from(DIR).join(playlist).with_extension(EXT)
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
pub struct Existing {
	pub q: HashMap<Q, HashSet<Source>>,
	pub playlist: HashMap<String, HashSet<Source>>,
}

pub fn existing() -> hmerr::Result<Existing> {
	let mut ret = Existing::default();

	let output = fs::read_dir(DIR).map_err(|e| ioe!(DIR, e))?;

	for entry in output {
		let entry = entry.map_err(|e| ioe!(DIR, e))?;

		let path = entry.path();
		if !path.is_file() || path.extension().map(|ext| ext.to_str()) != Some(Some(EXT)) {
			continue;
		}

		let Some(name) = path.file_stem() else {
			continue;
		};

		let list =
			parse_content(&fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?);

		let name = name.to_string_lossy();
		if name.starts_with(PREFIX) {
			let q = parse_q(&name)?;
			ret.q.insert(q, list);
		} else {
			ret.playlist.insert(name.to_string(), list);
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

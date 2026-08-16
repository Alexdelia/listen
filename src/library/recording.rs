use std::{
	collections::HashSet,
	path::{Path, PathBuf},
};

use crate::declaration::Source;

use super::file;

pub const DIR: &str = "./output/recording";

pub const EXT: &str = "mp3";

pub fn path(source: Source) -> PathBuf {
	Path::new(DIR).join(source.to_string()).with_extension(EXT)
}

pub fn existing() -> hmerr::Result<HashSet<Source>> {
	Ok(file::with_extension(DIR, EXT)?
		.iter()
		.filter_map(|found| found.stem.parse().ok())
		.collect())
}

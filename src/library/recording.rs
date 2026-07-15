use std::{
	collections::HashSet,
	fs,
	path::{Path, PathBuf},
};

use hmerr::ioe;

use crate::declaration::Source;

pub const DIR: &str = "./output/recording";

pub const EXT: &str = "mp3";

pub fn path(source: &str) -> PathBuf {
	Path::new(DIR).join(source).with_extension(EXT)
}

pub fn existing() -> hmerr::Result<HashSet<Source>> {
	let output = fs::read_dir(DIR).map_err(|e| ioe!(DIR, e))?;
	let mut existing = HashSet::<Source>::new();

	for entry in output {
		let entry = entry.map_err(|e| ioe!(DIR, e))?;

		let path = entry.path();
		if !path.is_file() || path.extension().map(|ext| ext.to_str()) != Some(Some(EXT)) {
			continue;
		}

		let Some(source) = path.file_stem() else {
			continue;
		};

		existing.insert(source.to_string_lossy().to_string());
	}

	Ok(existing)
}

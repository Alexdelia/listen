use std::{fs, path::PathBuf};

use hmerr::ioe;

pub struct Named {
	pub path: PathBuf,
	pub stem: String,
}

pub fn with_extension(dir: &str, ext: &str) -> hmerr::Result<Vec<Named>> {
	let mut found = Vec::new();

	for entry in fs::read_dir(dir).map_err(|e| ioe!(dir, e))? {
		let path = entry.map_err(|e| ioe!(dir, e))?.path();

		if !path.is_file() || path.extension().map(|found| found.to_str()) != Some(Some(ext)) {
			continue;
		}

		let Some(stem) = path
			.file_stem()
			.map(|stem| stem.to_string_lossy().to_string())
		else {
			continue;
		};

		found.push(Named { path, stem });
	}

	Ok(found)
}

use std::{
	fs::{self, OpenOptions},
	io::Write,
	path::Path,
};

use hmerr::ioe;

use crate::prepare;

pub fn read(path: &Path) -> hmerr::Result<Option<String>> {
	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	let content = content.trim();

	Ok((!content.is_empty()).then(|| content.to_string()))
}

pub fn write(path: &Path, content: &str) -> hmerr::Result<()> {
	prepare(path)?;

	fs::write(path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

pub fn append(path: &Path, line: &str) -> hmerr::Result<()> {
	prepare(path)?;

	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(path)
		.map_err(|e| ioe!(path.to_string_lossy(), e))?;

	writeln!(file, "{line}").map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

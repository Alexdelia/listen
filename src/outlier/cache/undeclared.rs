use std::{fs, path::PathBuf};

use hmerr::ioe;

const SUBDIR: &str = "undeclared";
const EXT: &str = "txt";

pub(in crate::outlier) fn write(username: &str, content: &str) -> hmerr::Result<PathBuf> {
	let path = path(username)?;
	super::prepare(&path)?;

	fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(path)
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	Ok(super::root()?
		.join(SUBDIR)
		.join(username)
		.with_extension(EXT))
}

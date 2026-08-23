use std::{fs, path::PathBuf};

use hmerr::ioe;

const SUBDIR: &str = "undeclared";
const EXT: &str = "csv";

pub(in crate::outlier) fn write(username: &str, content: &str) -> hmerr::Result<PathBuf> {
	let path = listen_cache::path(SUBDIR, username, EXT)?;
	super::prepare(&path)?;

	fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(path)
}

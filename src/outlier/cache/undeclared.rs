use std::path::PathBuf;

use listen_cache::text;

const SUBDIR: &str = "undeclared";
const EXT: &str = "csv";

pub(in crate::outlier) fn write(username: &str, content: &str) -> hmerr::Result<PathBuf> {
	let path = listen_cache::path(SUBDIR, username, EXT)?;
	text::write(&path, content)?;

	Ok(path)
}

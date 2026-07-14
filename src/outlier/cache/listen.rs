use std::{fs, path::PathBuf};

use ansi::abbrev::R;
use hmerr::{ge, ioe};

use super::super::fetch::ListenCount;

const SUBDIR: &str = "listen";
const EXT: &str = "json";

pub(in crate::outlier) fn read(username: &str) -> hmerr::Result<Option<ListenCount>> {
	let path = path(username)?;

	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(serde_json::from_str(&content).ok())
}

pub(in crate::outlier) fn write(username: &str, listen: &ListenCount) -> hmerr::Result<()> {
	let path = path(username)?;
	super::prepare(&path)?;

	let content = serde_json::to_string(listen)
		.map_err(|e| ge!(format!("{R}failed to encode cache\n{e}")))?;

	fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	Ok(super::root()?
		.join(SUBDIR)
		.join(username)
		.with_extension(EXT))
}

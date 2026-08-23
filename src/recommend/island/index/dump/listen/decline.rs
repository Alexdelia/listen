use std::{fs, path::Path};

use hmerr::ioe;

const DECLINED: &str = "declined-full";

pub(crate) fn declined(root: &Path) -> Option<String> {
	fs::read_to_string(root.join(DECLINED))
		.ok()
		.map(|name| name.trim().to_string())
}

pub(crate) fn decline(root: &Path, dump: &str) -> hmerr::Result<()> {
	let path = root.join(DECLINED);
	fs::write(&path, dump).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

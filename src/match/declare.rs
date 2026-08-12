use std::path::Path;

use hmerr::ioe;

use super::output;

pub(super) fn run(path: &Path, mbid: &str, recommend: bool) -> hmerr::Result<bool> {
	if recommend && !ux::ask_yn("declare", true).map_err(|e| ioe!("stdin", e))? {
		return Ok(false);
	}

	output::entry(path, mbid)?;

	Ok(true)
}

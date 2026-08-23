use std::path::Path;

use crate::prompt;

use super::output;

pub(super) fn run(path: &Path, mbid: &str, recommend: bool) -> hmerr::Result<bool> {
	if recommend && !prompt::confirm("declare", true)? {
		return Ok(false);
	}

	output::entry(path, mbid)?;

	Ok(true)
}

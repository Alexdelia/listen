use std::path::Path;

use hmerr::ioe;

use super::{open, output, verify::Info};

pub(super) fn run(
	path: &Path,
	mbid: &str,
	found: Option<(&Info, &str)>,
	length: i64,
	recommend: bool,
) -> hmerr::Result<bool> {
	if let Some((info, url)) = found {
		output::found(info, length);
		output::url(url);

		if recommend {
			open::open(url)?;
		}
	}

	if recommend && !ux::ask_yn("declare", true).map_err(|e| ioe!("stdin", e))? {
		return Ok(false);
	}

	output::entry(path, mbid)?;

	Ok(true)
}

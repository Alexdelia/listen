use std::path::Path;

use hmerr::ioe;

use super::{find::Found, output};

pub(super) fn run(
	path: &Path,
	mbid: &str,
	found: &Found,
	length: i64,
	recommend: bool,
) -> hmerr::Result<bool> {
	output::found(&found.info, length);
	output::url(&found.url);

	output::musicbrainz(mbid, &found.url)?;

	if recommend && !ux::ask_yn("declare", true).map_err(|e| ioe!("stdin", e))? {
		return Ok(false);
	}
	output::entry(path, mbid)?;

	Ok(true)
}

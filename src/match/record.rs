use std::path::Path;

use super::{declare, find::Found, output};

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

	declare::run(path, mbid, recommend)
}

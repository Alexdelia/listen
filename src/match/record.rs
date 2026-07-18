use std::path::Path;

use super::{find::Found, output};

pub(super) fn run(path: &Path, mbid: &str, found: &Found, length: i64) -> hmerr::Result<()> {
	output::found(&found.info, length);
	output::url(&found.url);
	output::musicbrainz(mbid, &found.url)?;
	output::entry(path, mbid)
}

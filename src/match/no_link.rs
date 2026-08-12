use std::path::Path;

use hmerr::ioe;
use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use super::{declare, find, open, output};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	path: &Path,
	mbid: &str,
	recommend: bool,
) -> hmerr::Result<bool> {
	let found = find::song(client, recording, title, length, mbid).await?;

	output::found(&found.info, length);
	output::url(&found.url);
	open::open(&found.url)?;

	if !ux::ask_yn("does this song match", true).map_err(|e| ioe!("stdin", e))? {
		return Ok(false);
	}
	output::musicbrainz(mbid, &found.url)?;

	declare::run(path, mbid, recommend)
}

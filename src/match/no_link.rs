use std::path::Path;

use hmerr::ioe;
use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use super::{find, open, output};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	path: &Path,
	mbid: &str,
) -> hmerr::Result<()> {
	let found = find::song(client, recording, title, length).await?;

	output::found(&found.info, length);
	open::open(&found.url)?;

	if !ux::ask_yn("does this song match", true).map_err(|e| ioe!("stdin", e))? {
		return Ok(());
	}

	output::musicbrainz(mbid, &found.url)?;
	output::entry(path, mbid)
}

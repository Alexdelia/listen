use std::path::Path;

use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use super::{find, record};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	path: &Path,
	mbid: &str,
) -> hmerr::Result<()> {
	let found = find::song(client, recording, title, length).await?;

	record::run(path, mbid, &found, length)
}

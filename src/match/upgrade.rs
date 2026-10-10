use std::path::Path;

use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use super::{find, outcome::Outcome, record};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	path: &Path,
	mbid: &str,
	recommend: bool,
) -> hmerr::Result<Outcome> {
	let found = find::song(client, recording, title, length, mbid).await?;

	Ok(Outcome::of(
		record::run(path, mbid, &found, length, recommend)?,
		Some(&found.url),
	))
}

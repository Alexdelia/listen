use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use super::{find, output};

pub(super) async fn run(
	client: &MusicBrainzClient,
	recording: &Recording,
	title: &str,
	length: i64,
	mbid: &str,
) -> hmerr::Result<()> {
	let found = find::song(client, recording, title, length).await?;

	output::found(&found.info, length);
	output::musicbrainz(mbid, &found.url)?;
	output::entry(mbid);
	Ok(())
}

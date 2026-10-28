use std::path::Path;

use musicbrainz_rs::{MusicBrainzClient, entity::recording::Recording};

use crate::prompt;

use super::{declare, find, open, outcome::Outcome, output};

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

	output::found(&found.info, length);
	output::url(&found.url);
	open::open(&found.url)?;

	if !prompt::confirm("song match", true)? {
		return Ok(Outcome::Declined);
	}
	output::musicbrainz(mbid, &found.url)?;

	Ok(Outcome::of(
		declare::run(path, mbid, recommend)?,
		Some(&found.url),
	))
}

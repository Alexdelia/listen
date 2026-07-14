mod duration;
mod find;
mod keep;
mod link;
mod no_link;
mod open;
mod output;
mod upgrade;
mod verify;

use std::path::Path;

use ansi::abbrev::{B, D, R};
use musicbrainz_rs::{Fetch, MusicBrainzClient, entity::recording::Recording};

use crate::MUSIC_BRAINZ_USER_AGENT;

pub async fn run(path: &Path, mbid: &str) -> hmerr::Result<()> {
	let client = MusicBrainzClient::new(MUSIC_BRAINZ_USER_AGENT);

	let recording = Recording::fetch()
		.id(mbid)
		.with_artists()
		.with_aliases()
		.with_url_relations()
		.execute_with_client_async(&client)
		.await
		.map_err(|e| format!("{R}failed to fetch recording {B}{mbid}{D}\n{e:#?}"))?;

	let title = recording.title.trim().to_string();
	if title.is_empty() {
		return Err(format!("{R}recording {B}{mbid}{D} has no title").into());
	}

	let Some(length) = recording.length else {
		return Err(format!(
			"{R}recording {B}{title}{D} has no length, cannot confirm match by duration{D}"
		)
		.into());
	};
	let length = duration::round_sec(length);

	let Some(id) = link::youtube(&recording) else {
		return no_link::run(&client, &recording, &title, length, path, mbid).await;
	};

	match verify::verify(&id)? {
		None => todo!("reverse-engineer the music.youtube.com dead-link redirect"),
		Some(info) if info.is_song() => keep::run(path, mbid, &info, length),
		Some(_video) => upgrade::run(&client, &recording, &title, length, path, mbid).await,
	}
}

mod declare;
mod duration;
mod find;
mod keep;
mod link;
mod linked;
mod no_link;
mod output;
mod record;
mod redirect;
mod upgrade;
mod verify;

use std::path::Path;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use musicbrainz_rs::{Fetch, entity::recording::Recording};

use crate::{music_brainz, open};

pub(crate) fn declare(path: &Path, mbid: &str) -> hmerr::Result<bool> {
	declare::run(path, mbid, true)
}

pub(crate) fn is_song(url: &str) -> hmerr::Result<bool> {
	let Some(id) = link::video_id(url) else {
		return Ok(false);
	};

	Ok(verify::verify(&id)?.is_some_and(|info| info.is_song()))
}

pub(crate) async fn run(path: &Path, mbid: &str, recommend: bool) -> hmerr::Result<bool> {
	let client = music_brainz::client();

	let recording = Recording::fetch()
		.id(mbid)
		.with_artists()
		.with_aliases()
		.with_url_relations()
		.execute_with_client_async(&client)
		.await
		.map_err(|e| ge!(format!("{R}failed to fetch recording {B}{mbid}{D}\n{e:#?}")))?;

	let title = recording.title.trim().to_string();
	if title.is_empty() {
		return Err(ge!(format!("{R}recording {B}{mbid}{D} has no title")).into());
	}

	let Some(length) = recording.length else {
		return Err(ge!(format!(
			"{R}recording {B}{title}{D} has no length, cannot confirm match by duration{D}"
		))
		.into());
	};
	let length = duration::round_sec(length);

	output::recording(&recording, &title, length);

	match link::streaming(&recording) {
		None => no_link::run(&client, &recording, &title, length, path, mbid, recommend).await,
		Some(already @ (link::Streaming::SoundCloud | link::Streaming::Bandcamp)) => {
			println!(
				"{B}{name}{D} link already on musicbrainz",
				name = already.name()
			);
			keep::run(path, mbid, None, length, recommend)
		}
		Some(link::Streaming::YouTubeMusic(ids)) => match linked::resolve(ids)? {
			linked::Linked::Song(found) => keep::run(
				path,
				mbid,
				Some((&found.info, &found.url)),
				length,
				recommend,
			),
			linked::Linked::Replacement(found) => {
				record::run(path, mbid, &found, length, recommend)
			}
			linked::Linked::Video => {
				upgrade::run(&client, &recording, &title, length, path, mbid, recommend).await
			}
			linked::Linked::Dead => {
				no_link::run(&client, &recording, &title, length, path, mbid, recommend).await
			}
		},
	}
}

mod declare;
mod declared;
mod duration;
mod find;
mod keep;
mod link;
mod linked;
mod no_link;
mod outcome;
mod output;
mod record;
mod redirect;
mod upgrade;
mod verify;

use std::path::Path;

use console::style;
use miette::miette;
use musicbrainz_rs::{Fetch, entity::recording::Recording};

use crate::{music_brainz, open, tag_suggestion};

use outcome::Outcome;

pub(crate) fn declare(path: &Path, mbid: &str) -> miette::Result<bool> {
	declare::run(path, mbid, true)
}

pub(crate) fn is_song(url: &str) -> miette::Result<bool> {
	let Some(id) = link::video_id(url) else {
		return Ok(false);
	};

	Ok(verify::verify(&id)?.is_some_and(|info| info.is_song()))
}

pub(crate) async fn run(path: &Path, mbid: &str, recommend: bool) -> miette::Result<bool> {
	let client = music_brainz::client();

	let recording = Recording::fetch()
		.id(mbid)
		.with_artists()
		.with_aliases()
		.with_genres()
		.with_tags()
		.with_url_relations()
		.execute_with_client_async(&client)
		.await
		.map_err(|e| miette!("failed to fetch recording {mbid}\n{e:#?}"))?;

	let title = recording.title.trim().to_string();
	if title.is_empty() {
		return Err(miette!("recording {mbid} has no title"));
	}

	let Some(length) = recording.length else {
		return Err(miette!(
			"recording {title} has no length, cannot confirm match by duration"
		));
	};
	let length = duration::round_sec(length);

	output::recording(&recording, &title, length);

	let outcome = match link::streaming(&recording) {
		None => no_link::run(&client, &recording, &title, length, path, mbid, recommend).await?,
		Some(already @ (link::Streaming::SoundCloud | link::Streaming::Bandcamp)) => {
			println!(
				"{} link already on musicbrainz",
				style(&already.name()).bold()
			);
			Outcome::of(
				keep::run(path, mbid, None, length, recommend)?,
				link::url(&recording, &already),
			)
		}
		Some(link::Streaming::YouTubeMusic(ids)) => match linked::resolve(ids)? {
			linked::Linked::Song(found) => Outcome::of(
				keep::run(
					path,
					mbid,
					Some((&found.info, &found.url)),
					length,
					recommend,
				)?,
				Some(&found.url),
			),
			linked::Linked::Replacement(found) => Outcome::of(
				record::run(path, mbid, &found, length, recommend)?,
				Some(&found.url),
			),
			linked::Linked::Video => {
				upgrade::run(&client, &recording, &title, length, path, mbid, recommend).await?
			}
			linked::Linked::Dead => {
				no_link::run(&client, &recording, &title, length, path, mbid, recommend).await?
			}
		},
	};

	let Outcome::Declared { url } = outcome else {
		return Ok(false);
	};

	tag_suggestion::suggest(&recording, url.as_deref());

	Ok(true)
}

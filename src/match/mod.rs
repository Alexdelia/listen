mod duration;
mod form;
mod matching;
mod query;
mod search;
mod text;
mod verify;

use ansi::abbrev::{B, D, R};
use musicbrainz_rs::{Fetch, MusicBrainzClient, entity::recording::Recording};

use crate::MUSIC_BRAINZ_USER_AGENT;

const WATCH_BASE: &str = "https://music.youtube.com/watch?v=";
const MAX_CANDIDATE: usize = 10;

pub async fn run(mbid: &str) -> hmerr::Result<()> {
	let client = MusicBrainzClient::new(MUSIC_BRAINZ_USER_AGENT);

	let recording = Recording::fetch()
		.id(mbid)
		.with_artists()
		.with_aliases()
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

	let title_form = form::title(&recording, &title);
	let artist = form::artist(&client, &recording).await;
	let accepted = form::accepted_title(&title_form);

	let candidate = search::search(&query::build(&title_form, &artist))?;
	if candidate.is_empty() {
		return Err(format!("{R}no YouTube Music song result for {B}{title}{D}").into());
	}

	for id in candidate {
		let Some(info) = verify::verify(&id)? else {
			continue;
		};

		if matching::is_match(&accepted, length, &info) {
			println!("{WATCH_BASE}{id}");
			eprintln!(
				"{B}{track}{D} - {artist}, {dur} ({delta:+}s){album}",
				track = info.track.as_deref().unwrap_or("?"),
				artist = info.artist.as_deref().unwrap_or("?"),
				dur = duration::fmt(info.duration),
				delta = info.duration.unwrap_or(length) - length,
				album = info.album.map(|a| format!(", {a}")).unwrap_or_default(),
			);

			return Ok(());
		}
	}

	Err(format!(
		"{R}no exact YouTube Music match for {B}{title}{D} among {MAX_CANDIDATE} song result{D}"
	)
	.into())
}

fn push_unique(out: &mut Vec<String>, s: String) {
	if !s.is_empty() && !out.contains(&s) {
		out.push(s);
	}
}

mod duration;
mod find;
mod keep;
mod link;
mod no_link;
mod output;
mod record;
mod redirect;
mod upgrade;
mod verify;

use std::{collections::HashSet, ops::ControlFlow, path::Path};

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use musicbrainz_rs::{Fetch, entity::recording::Recording};

use crate::{music_brainz, open};

#[derive(Clone, Copy)]
pub enum Prompt {
	Confirm,
	Review,
}

pub async fn run(path: &Path, mbid: &str, prompt: Prompt) -> hmerr::Result<ControlFlow<()>> {
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

	match link::streaming(&recording) {
		None => no_link::run(&client, &recording, &title, length, path, mbid, prompt).await,
		Some(link::Streaming::SoundCloud) => {
			eprintln!("{B}soundcloud{D} link already on musicbrainz");
			keep::run(path, mbid, None, length)?;
			Ok(ControlFlow::Continue(()))
		}
		Some(link::Streaming::YouTubeMusic(mut id)) => {
			let mut dead = HashSet::new();

			loop {
				match verify::verify(&id)? {
					Some(info) if info.is_song() => {
						if dead.is_empty() {
							keep::run(path, mbid, Some(&info), length)?;
						} else {
							let found = find::Found {
								url: verify::watch(&id),
								info,
							};
							record::run(path, mbid, &found, length)?;
						}
						break Ok(ControlFlow::Continue(()));
					}
					Some(_video) => {
						upgrade::run(&client, &recording, &title, length, path, mbid).await?;
						break Ok(ControlFlow::Continue(()));
					}
					None => {
						dead.insert(id.clone());

						match redirect::resolve(&id)? {
							Some(replacement) if !dead.contains(&replacement) => id = replacement,
							_ => {
								break no_link::run(
									&client, &recording, &title, length, path, mbid, prompt,
								)
								.await;
							}
						}
					}
				}
			}
		}
	}
}

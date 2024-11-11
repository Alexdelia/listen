mod streaming_source;

use std::borrow::Borrow;
use std::collections::HashSet;
use std::path::PathBuf;

use ansi::abbrev::{B, D, R, Y};

use async_std::{channel::Sender, task};
use id3::{Tag, TagLike};
use musicbrainz_rs_nova::{
	entity::{recording::Recording, relations::RelationContent},
	Fetch,
};

use streaming_source::StreamingSource;

use crate::channel::{Action, Status};
use crate::entry::Entry;
use crate::MUSIC_BRAINZ_USER_AGENT;

pub async fn fetch(sync: &[String], tx: Sender<Status>) {
	musicbrainz_rs_nova::config::set_user_agent(MUSIC_BRAINZ_USER_AGENT);

	let mut handles = vec![];

	for entry in sync {
		let res = Recording::fetch()
			.id(entry)
			.with_artists()
			.with_genres()
			.with_tags()
			.with_url_relations()
			.execute()
			.await;

		let Ok(recording) = res else {
			tx.send(Status {
				action: Action::FetchMusicBrainz,
				status: Err(format!("{R}failed to fetch {B}{entry}{D}\n{res:#?}")),
			})
			.await
			.expect("failed to send fetch status");

			continue;
		};

		tx.send(Status {
			action: Action::FetchMusicBrainz,
			status: Ok(()),
		})
		.await
		.expect("failed to send fetch status");

		let entry = entry.clone();

		let tcx = tx.clone();
		handles.push(task::spawn(async move {
			let Some(path) = fetch_recording(&entry, &recording, &tcx).await else {
				return;
			};

			add_metadata(path, recording, &tcx).await;
		}));
	}

	for handle in handles {
		handle.await;
	}
}

async fn fetch_recording(
	entry: &str,
	recording: &Recording,
	tx: &Sender<Status>,
) -> Option<PathBuf> {
	let title = &recording.title;

	let Some(relations) = &recording.relations else {
		tx.send(Status {
			action: Action::FetchStreaming,
			status: Err(format!("{R}no relations for {B}{entry} ({title}){D}")),
		})
		.await
		.expect("failed to send fetch streaming status");

		return None;
	};

	let mut urls = Vec::with_capacity(4);

	for relation in relations {
		if relation.relation_type != "free streaming" {
			continue;
		}

		let RelationContent::Url(url) = &relation.content else {
			continue;
		};

		let Ok(streaming_source) = StreamingSource::try_from(url.borrow()) else {
			continue;
		};

		urls.push((streaming_source, url.resource.clone()));
	}

	if urls.is_empty() {
		tx.send(Status {
			action: Action::FetchStreaming,
			status: Err(format!(
				"{R}no streaming urls for {B}{entry} ({title}){D}\n{Y}{relations:#?}{D}"
			)),
		})
		.await
		.expect("failed to send fetch streaming status");

		return None;
	}

	let url = urls.first().unwrap();
	let path = Entry::path_from_source(entry);

	match url.0.download(&url.1, &path).map_err(|e| e.to_string()) {
		Ok(_) => tx
			.send(Status {
				action: Action::FetchStreaming,
				status: Ok(()),
			})
			.await
			.expect("failed to send fetch streaming status"),
		Err(e) => {
			tx.send(Status {
				action: Action::FetchStreaming,
				status: Err(format!(
					"{R}failed to download {B}{entry} ({title}){D}\n{e}"
				)),
			})
			.await
			.expect("failed to send fetch streaming status");

			return None;
		}
	}

	Some(path)
}

async fn add_metadata(path: PathBuf, recording: Recording, tx: &Sender<Status>) {
	let mut tag = Tag::read_from_path(&path).unwrap_or_default();

	let Recording {
		title,
		artist_credit,
		genres,
		tags,
		..
	} = recording;

	if !title.is_empty() {
		tag.set_title(title);
	}

	if let Some(artist_credit) = artist_credit {
		if !artist_credit.is_empty() {
			let artists = artist_credit
				.iter()
				.map(|ac| ac.artist.name.as_str())
				.collect::<Vec<_>>()
				.join(" & ");

			tag.set_artist(artists);
		};
	}

	let mut all_tags = HashSet::new();

	let genres_binding = genres.unwrap_or_default();
	all_tags.extend(genres_binding.iter().map(|g| g.name.as_str()));
	let tags_binding = tags.unwrap_or_default();
	all_tags.extend(tags_binding.iter().map(|t| t.name.as_str()));

	if !all_tags.is_empty() {
		let mut tags = all_tags.into_iter().collect::<Vec<_>>();
		tags.sort_unstable();

		tag.set_genre(tags.join(" / "));
	}

	match tag.write_to_path(&path, id3::Version::default()) {
		Ok(_) => tx.send(Status {
			action: Action::AddMetadata,
			status: Ok(()),
		}),
		Err(e) => tx.send(Status {
			action: Action::AddMetadata,
			status: Err(format!(
				"{R}failed to write metadata to {B}{path}{D}\n{e}",
				path = path.to_string_lossy()
			)),
		}),
	}
	.await
	.expect("failed to send add metadata status");
}

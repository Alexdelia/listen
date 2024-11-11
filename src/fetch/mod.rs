mod streaming_source;

use std::borrow::Borrow;

use ansi::abbrev::{B, D, R, Y};

use async_std::channel::{Receiver, Sender};
use async_std::path::{Path, PathBuf};
use async_std::task;
use id3::{Tag, TagLike};
use musicbrainz_rs_nova::entity::recording::Recording;
use musicbrainz_rs_nova::entity::relations::RelationContent;
use musicbrainz_rs_nova::Fetch;
use streaming_source::StreamingSource;

use crate::channel::{Action, Status};
use crate::entry::Entry;
use crate::{filter::SyncEntry, MUSIC_BRAINZ_USER_AGENT};

pub async fn fetch(sync: &SyncEntry, tx: Sender<Status>) {
	musicbrainz_rs_nova::config::set_user_agent(MUSIC_BRAINZ_USER_AGENT);

	let mut handles = vec![];

	for entry in &sync.add {
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
	let path = Path::new(Entry::OUTPUT_DIR)
		.join(entry)
		.with_extension(Entry::EXT);

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
	/*
	let mut tag = Tag::read_from

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
	*/

	tx.send(Status {
		action: Action::AddMetadata,
		status: Ok(()),
	})
	.await
	.expect("failed to send add metadata status");
}

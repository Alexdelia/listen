mod streaming_source;

use std::borrow::Borrow;
use std::process::Command;

use ansi::abbrev::{B, D, R, Y};

use async_std::path::Path;
use async_std::task;
use musicbrainz_rs_nova::entity::relations::RelationContent;
use musicbrainz_rs_nova::entity::{recording::Recording, url::Url};
use musicbrainz_rs_nova::Fetch;
use streaming_source::StreamingSource;

use crate::entry::Entry;
use crate::{env, filter::SyncEntry, MUSIC_BRAINZ_USER_AGENT};

pub async fn fetch(sync: &SyncEntry) {
	musicbrainz_rs_nova::config::set_user_agent(MUSIC_BRAINZ_USER_AGENT);

	let mut handles = vec![];

	for entry in &sync.add {
		let res = Recording::fetch()
			.id(entry)
			.with_artists()
			.with_genres()
			.with_tags()
			// .with_releases()
			// .with_medias()
			// .with_work_level_relations()
			// .with_work_relations()
			// .with_aliases()
			.with_url_relations()
			.execute()
			.await;

		let Ok(recording) = res else {
			eprintln!("{R}failed to fetch {B}{entry}{D}\n{res:#?}");
			continue;
		};

		let entry = entry.clone();

		handles.push(task::spawn(async move {
			fetch_recording(entry, recording).await
		}));
	}

	for handle in handles {
		handle.await;
	}
}

async fn fetch_recording(entry: String, recording: Recording) {
	let title = recording.title;

	let Some(relations) = recording.relations else {
		eprintln!("{R}no relations for {B}{entry} ({title}){D}");
		return;
	};

	let mut urls = Vec::with_capacity(4);

	for relation in &relations {
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
		eprintln!("{R}no streaming urls for {B}{entry} ({title}){D}\n{Y}{relations:#?}{D}");
		return;
	}

	let url = urls.first().unwrap();
	url.0.download(
		&url.1,
		Path::new(Entry::OUTPUT_DIR)
			.join(entry)
			.with_extension(Entry::EXT),
	);

	// need to add metadata to downloaded file
}

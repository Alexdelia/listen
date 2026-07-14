use std::{collections::HashSet, path::Path};

use ansi::abbrev::{B, D, R};
use id3::{Tag, TagLike, Version, frame::ExtendedText};
use musicbrainz_rs::entity::recording::Recording;

const RECORDING_MBID: &str = "MusicBrainz Track Id";

pub fn write(path: &Path, recording: &Recording) -> Result<(), String> {
	let mut tag = Tag::read_from_path(path).unwrap_or_default();

	if !recording.title.is_empty() {
		tag.set_title(recording.title.as_str());
	}

	if let Some(artist_credit) = &recording.artist_credit
		&& !artist_credit.is_empty()
	{
		let artist = artist_credit
			.iter()
			.map(|ac| ac.artist.name.as_str())
			.collect::<Vec<_>>()
			.join(" & ");

		tag.set_artist(artist);
	}

	let genre = genre(recording);
	if !genre.is_empty() {
		tag.set_genre(genre);
	}

	tag.add_frame(ExtendedText {
		description: RECORDING_MBID.to_string(),
		value: recording.id.clone(),
	});

	tag.write_to_path(path, Version::default()).map_err(|e| {
		format!(
			"{R}failed to write metadata to {B}{path}{D}\n{e}",
			path = path.to_string_lossy(),
		)
	})
}

fn genre(recording: &Recording) -> String {
	let mut all = HashSet::new();

	if let Some(genres) = &recording.genres {
		all.extend(genres.iter().map(|g| g.name.as_str()));
	}
	if let Some(tags) = &recording.tags {
		all.extend(tags.iter().map(|t| t.name.as_str()));
	}

	let mut all = all.into_iter().collect::<Vec<_>>();
	all.sort_unstable();

	all.join(" / ")
}

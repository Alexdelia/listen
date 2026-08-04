use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct CreatedFor {
	pub playlists: Vec<Wrapper>,
}

#[derive(Deserialize)]
pub(super) struct Wrapper {
	pub playlist: Playlist,
}

#[derive(Deserialize)]
pub(super) struct Playlist {
	pub date: DateTime<Utc>,
	pub identifier: String,
	extension: Extension,
	#[serde(default)]
	pub track: Vec<Track>,
}

#[derive(Deserialize)]
pub(super) struct Track {
	#[serde(default)]
	pub identifier: Vec<String>,
}

impl Playlist {
	pub(super) fn source_patch(&self) -> Option<&str> {
		self.extension
			.playlist
			.additional_metadata
			.algorithm_metadata
			.source_patch
			.as_deref()
	}
}

#[derive(Default, Deserialize)]
struct Extension {
	#[serde(default, rename = "https://musicbrainz.org/doc/jspf#playlist")]
	playlist: PlaylistExtension,
}

#[derive(Default, Deserialize)]
struct PlaylistExtension {
	#[serde(default)]
	additional_metadata: AdditionalMetadata,
}

#[derive(Default, Deserialize)]
struct AdditionalMetadata {
	#[serde(default)]
	algorithm_metadata: AlgorithmMetadata,
}

#[derive(Default, Deserialize)]
struct AlgorithmMetadata {
	source_patch: Option<String>,
}

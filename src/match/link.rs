use musicbrainz_rs::entity::{recording::Recording, relations::RelationContent};

use crate::fetch::streaming_source::StreamingSource;

const FREE_STREAMING: &str = "free streaming";

pub(super) enum Streaming {
	SoundCloud,
	YouTubeMusic(String),
}

impl Streaming {
	fn priority(&self) -> u8 {
		match self {
			Self::SoundCloud => StreamingSource::SoundCloud.priority(),
			Self::YouTubeMusic(_) => StreamingSource::YouTubeMusic.priority(),
		}
	}
}

pub(super) fn streaming(recording: &Recording) -> Option<Streaming> {
	recording
		.relations
		.iter()
		.flatten()
		.filter(|relation| relation.relation_type == FREE_STREAMING)
		.filter_map(|relation| match &relation.content {
			RelationContent::Url(url) => classify(&url.resource),
			_ => None,
		})
		.min_by_key(Streaming::priority)
}

fn classify(url: &str) -> Option<Streaming> {
	match StreamingSource::try_from(url).ok()? {
		StreamingSource::SoundCloud => Some(Streaming::SoundCloud),
		StreamingSource::YouTubeMusic => video_id(url).map(Streaming::YouTubeMusic),
		StreamingSource::YouTube => None,
	}
}

pub(super) fn video_id(url: &str) -> Option<String> {
	let query = url
		.strip_prefix(StreamingSource::YouTubeMusic.base_url().as_str())?
		.strip_prefix("/watch?")?;

	query
		.split('&')
		.find_map(|param| param.strip_prefix("v="))
		.map(str::to_string)
}

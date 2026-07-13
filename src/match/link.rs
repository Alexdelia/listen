use musicbrainz_rs::entity::{recording::Recording, relations::RelationContent};

use crate::fetch::streaming_source::StreamingSource;

const FREE_STREAMING: &str = "free streaming";

pub(super) fn youtube(recording: &Recording) -> Option<String> {
	recording
		.relations
		.iter()
		.flatten()
		.filter(|relation| relation.relation_type == FREE_STREAMING)
		.filter_map(|relation| match &relation.content {
			RelationContent::Url(url) => Some(&url.resource),
			_ => None,
		})
		.find_map(|url| video_id(url))
}

fn video_id(url: &str) -> Option<String> {
	let rest = url
		.strip_prefix("https://")
		.or_else(|| url.strip_prefix("http://"))?;
	let query = rest
		.strip_prefix(StreamingSource::YouTubeMusic.host())?
		.strip_prefix("/watch?")?;

	query
		.split('&')
		.find_map(|param| param.strip_prefix("v="))
		.map(str::to_string)
}

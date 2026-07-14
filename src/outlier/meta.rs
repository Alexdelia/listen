use id3::{Tag, TagLike};

use crate::entry::Entry;

pub(super) fn label(mbid: &str) -> String {
	let Ok(tag) = Tag::read_from_path(Entry::path_from_source(mbid)) else {
		return String::new();
	};

	let title = tag.title().unwrap_or_default().trim();
	let artist = tag.artist().unwrap_or_default().trim();

	match (title.is_empty(), artist.is_empty()) {
		(true, true) => String::new(),
		(false, true) => title.to_string(),
		(true, false) => artist.to_string(),
		(false, false) => format!("{title} - {artist}"),
	}
}

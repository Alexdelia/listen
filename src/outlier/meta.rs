use id3::{Tag, TagLike};

use crate::entry::Entry;

pub(super) fn label(mbid: &str) -> String {
	let Ok(tag) = Tag::read_from_path(Entry::path_from_source(mbid)) else {
		return String::new();
	};

	join(
		tag.title().unwrap_or_default(),
		tag.artist().unwrap_or_default(),
	)
}

pub(super) fn join(title: &str, artist: &str) -> String {
	let title = title.trim();
	let artist = artist.trim();

	match (title.is_empty(), artist.is_empty()) {
		(true, true) => String::new(),
		(false, true) => title.to_string(),
		(true, false) => artist.to_string(),
		(false, false) => format!("{title} - {artist}"),
	}
}

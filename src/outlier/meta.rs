use std::collections::HashMap;

use ansi::{DIM, abbrev::D};
use id3::{Tag, TagLike};

use crate::{
	declaration::{Entry, Source},
	library,
};

pub(super) type Meta = HashMap<Source, (String, String)>;

pub(super) fn declared(list: &[Entry]) -> Meta {
	list.iter()
		.filter_map(|entry| read(entry.s).map(|title_artist| (entry.s, title_artist)))
		.collect()
}

pub(super) fn read(mbid: Source) -> Option<(String, String)> {
	let tag = Tag::read_from_path(library::recording::path(mbid)).ok()?;

	let title = tag.title().unwrap_or_default().trim().to_string();
	let artist = tag.artist().unwrap_or_default().trim().to_string();

	if title.is_empty() && artist.is_empty() {
		return None;
	}

	Some((title, artist))
}

pub(super) fn label(mbid: Source) -> String {
	read(mbid).map_or_else(String::new, |(title, artist)| join(&title, &artist))
}

pub(super) fn join(title: &str, artist: &str) -> String {
	let title = title.trim();
	let artist = artist.trim();

	match (title.is_empty(), artist.is_empty()) {
		(true, true) => String::new(),
		(false, true) => title.to_string(),
		(true, false) => artist.to_string(),
		(false, false) => format!("{title} {DIM}-{D} {artist}"),
	}
}

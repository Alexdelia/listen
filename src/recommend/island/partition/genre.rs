use id3::{Tag, TagLike};

use crate::{
	declaration::Source,
	library::{self, tag::GENRE_SEPARATOR},
};

pub(super) fn read(mbid: Source) -> Vec<String> {
	let Ok(tag) = Tag::read_from_path(library::recording::path(mbid)) else {
		return Vec::new();
	};

	let Some(genre) = tag.genre() else {
		return Vec::new();
	};

	genre
		.split(GENRE_SEPARATOR)
		.map(str::trim)
		.filter(|token| !token.is_empty())
		.map(str::to_lowercase)
		.collect()
}

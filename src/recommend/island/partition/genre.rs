use id3::{Tag, TagLike};

use crate::{declaration::Source, library};

const PLACEHOLDER: [&str; 4] = ["music", "other", "misc", "unknown"];
const ATTRIBUTE: char = ':';

pub(super) fn read(mbid: Source) -> Vec<String> {
	let Ok(tag) = Tag::read_from_path(library::recording::path(mbid)) else {
		return Vec::new();
	};

	tag.genres_parsed()
		.iter()
		.map(|genre| genre.trim().to_lowercase())
		.filter(|token| describes_a_genre(token))
		.collect()
}

fn describes_a_genre(token: &str) -> bool {
	!token.is_empty() && !token.contains(ATTRIBUTE) && !PLACEHOLDER.contains(&token)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_scene_is_a_genre() {
		assert!(describes_a_genre("touhou"));
		assert!(describes_a_genre("drum and bass"));
	}

	#[test]
	fn a_placeholder_names_nothing() {
		assert!(!describes_a_genre("music"));
		assert!(!describes_a_genre("unknown"));
		assert!(!describes_a_genre(""));
	}

	#[test]
	fn an_attribute_is_not_a_genre() {
		assert!(!describes_a_genre("meter:4/4"));
		assert!(!describes_a_genre("bpm:130"));
		assert!(!describes_a_genre("vocal:true"));
	}
}

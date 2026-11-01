use std::fmt::Write;

use crate::{declaration::Source, music_brainz};

const ENDPOINT: &str = "https://musicbrainz.org/ws/2/tag";

pub(super) fn submit(bearer: &str, source: Source, genres: &[&str]) -> miette::Result<()> {
	music_brainz::submit(bearer, ENDPOINT, &recording_list(source, genres), "tag")
}

fn recording_list(source: Source, genres: &[&str]) -> String {
	let mut tags = String::new();
	for genre in genres {
		let _ = write!(
			tags,
			"<user-tag vote=\"upvote\"><name>{}</name></user-tag>",
			escaped(genre)
		);
	}

	format!("<recording id=\"{source}\"><user-tag-list>{tags}</user-tag-list></recording>")
}

fn escaped(text: &str) -> String {
	text.chars().fold(String::new(), |mut out, c| {
		match c {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'"' => out.push_str("&quot;"),
			'\'' => out.push_str("&apos;"),
			c => out.push(c),
		}
		out
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn source() -> Source {
		"7b5a3a8f-5e1d-4c43-9a1e-3f2d1c0b9a87"
			.parse()
			.expect("a valid mbid parses")
	}

	#[test]
	fn every_checked_genre_is_sent_as_its_own_upvote_on_the_recording() {
		let body = recording_list(source(), &["vaporwave", "future funk"]);

		assert!(body.contains("<recording id=\"7b5a3a8f-5e1d-4c43-9a1e-3f2d1c0b9a87\">"));
		assert!(body.contains("<user-tag vote=\"upvote\"><name>vaporwave</name></user-tag>"));
		assert!(body.contains("<user-tag vote=\"upvote\"><name>future funk</name></user-tag>"));
		assert!(!body.contains("downvote"));
	}

	#[test]
	fn a_genre_name_carrying_markup_is_escaped_rather_than_breaking_the_xml() {
		assert_eq!(escaped("r&b"), "r&amp;b");
		assert_eq!(escaped("<\"'>"), "&lt;&quot;&apos;&gt;");
	}
}

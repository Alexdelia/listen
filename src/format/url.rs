use std::ops::Range;

use console::Style;
use uuid::Uuid;

const MUSIC_BRAINZ: &str = "https://musicbrainz.org/";

pub(crate) fn url(url: &str) -> String {
	paint(url, &Style::new().cyan())
}

fn paint(url: &str, plain: &Style) -> String {
	let Some(id) = music_brainz_id(url) else {
		return plain.apply_to(url).to_string();
	};
	let (head, rest) = url.split_at(id.start);
	let (id, tail) = rest.split_at(id.len());
	let emphasis = plain.clone().bold();

	[(head, plain), (id, &emphasis), (tail, plain)]
		.into_iter()
		.filter(|(part, _)| !part.is_empty())
		.map(|(part, style)| style.apply_to(part).to_string())
		.collect()
}

fn music_brainz_id(url: &str) -> Option<Range<usize>> {
	let path = url.strip_prefix(MUSIC_BRAINZ)?;
	let mut start = MUSIC_BRAINZ.len();

	for segment in path.split('/') {
		if Uuid::try_parse(segment).is_ok() {
			return Some(start..start + segment.len());
		}
		start += segment.len() + 1;
	}

	None
}

#[cfg(test)]
mod tests {
	use console::{Style, strip_ansi_codes};

	use super::*;

	const ID: &str = "07070707-0707-0707-0707-070707070707";

	fn plain() -> Style {
		Style::new().cyan().force_styling(true)
	}

	#[test]
	fn a_musicbrainz_url_bolds_its_uuid_only() {
		let url = format!("https://musicbrainz.org/recording/{ID}/edit");

		assert_eq!(
			paint(&url, &plain()),
			format!(
				"{}{}{}",
				plain().apply_to("https://musicbrainz.org/recording/"),
				plain().bold().apply_to(ID),
				plain().apply_to("/edit"),
			)
		);
	}

	#[test]
	fn a_uuid_ending_the_url_leaves_no_empty_tail() {
		let url = format!("https://musicbrainz.org/recording/{ID}");

		assert_eq!(
			paint(&url, &plain()),
			format!(
				"{}{}",
				plain().apply_to("https://musicbrainz.org/recording/"),
				plain().bold().apply_to(ID),
			)
		);
	}

	#[test]
	fn a_musicbrainz_url_without_a_uuid_is_plain() {
		let url = "https://musicbrainz.org/oauth2/authorize?client_id=x";

		assert_eq!(paint(url, &plain()), plain().apply_to(url).to_string());
	}

	#[test]
	fn a_uuid_outside_musicbrainz_is_not_bold() {
		let url = format!("https://listenbrainz.org/playlist/{ID}");

		assert_eq!(paint(&url, &plain()), plain().apply_to(&url).to_string());
	}

	#[test]
	fn the_text_of_the_url_is_kept() {
		let url = format!("https://musicbrainz.org/recording/{ID}/edit");

		assert_eq!(strip_ansi_codes(&super::url(&url)), url);
	}
}

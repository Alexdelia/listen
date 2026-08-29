use id3::{Tag, TagLike};

use crate::declaration::Source;

use super::recording;

pub(crate) const ARTIST_SEPARATOR: &str = " & ";

pub(crate) const TITLE_SORT: &str = "TSOT";
pub(crate) const ARTIST_SORT: &str = "TSOP";

pub(crate) type Sort = (bool, String, bool, String);

pub(crate) fn sort(source: Source) -> Sort {
	of(&Tag::read_from_path(recording::path(source)).unwrap_or_default())
}

fn of(tag: &Tag) -> Sort {
	let artist = folded(text(tag, ARTIST_SORT).or_else(|| tag.artist()));
	let title = folded(text(tag, TITLE_SORT).or_else(|| tag.title()));

	(artist.is_empty(), artist, title.is_empty(), title)
}

fn text<'a>(tag: &'a Tag, id: &str) -> Option<&'a str> {
	tag.get(id).and_then(|frame| frame.content().text())
}

fn folded(text: Option<&str>) -> String {
	text.unwrap_or_default().trim().to_lowercase()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tag(artist: &str, artist_sort: &str, title: &str, title_sort: &str) -> Tag {
		let mut tag = Tag::new();

		tag.set_artist(artist);
		tag.set_title(title);
		if !artist_sort.is_empty() {
			tag.set_text(ARTIST_SORT, artist_sort);
		}
		if !title_sort.is_empty() {
			tag.set_text(TITLE_SORT, title_sort);
		}

		tag
	}

	#[test]
	fn the_sort_name_is_what_a_tagged_recording_sorts_by() {
		assert_eq!(
			of(&tag(
				"結束バンド",
				"Kessoku Band",
				"ひみつ基地",
				"himitsukichi"
			)),
			(
				false,
				"kessoku band".to_string(),
				false,
				"himitsukichi".to_string()
			)
		);
	}

	#[test]
	fn a_recording_carrying_no_sort_name_sorts_by_the_name_it_has() {
		assert_eq!(
			of(&tag("Alan Walker", "", "Ignite", "")),
			(
				false,
				"alan walker".to_string(),
				false,
				"ignite".to_string()
			)
		);
	}

	#[test]
	fn the_artist_orders_before_the_title() {
		let mut list = vec![
			of(&tag("b", "", "a", "")),
			of(&tag("a", "", "z", "")),
			of(&tag("a", "", "b", "")),
		];
		list.sort();

		assert_eq!(
			list,
			vec![
				of(&tag("a", "", "b", "")),
				of(&tag("a", "", "z", "")),
				of(&tag("b", "", "a", "")),
			]
		);
	}

	#[test]
	fn a_recording_nothing_names_sorts_last() {
		let untagged = of(&Tag::default());
		let mut list = vec![untagged.clone(), of(&tag("a", "", "b", ""))];
		list.sort();

		assert_eq!(list, vec![of(&tag("a", "", "b", "")), untagged]);
	}
}

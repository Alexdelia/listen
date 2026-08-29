use std::io;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use id3::{Error, ErrorKind, Tag, TagLike};

use crate::declaration::Source;

use super::recording;

const ARTIST_SEPARATOR: &str = " & ";

pub(crate) const ARTIST: &str = "TPE1";
pub(crate) const TITLE_SORT: &str = "TSOT";
pub(crate) const ARTIST_SORT: &str = "TSOP";

pub(crate) type Sort = (bool, String, bool, String);

pub(crate) fn sort(source: Source) -> hmerr::Result<Sort> {
	let path = recording::path(source);
	let path = path.to_string_lossy();

	match Tag::read_from_path(path.as_ref()) {
		Ok(tag) => Ok(of(&tag)),
		Err(e) if nameless(&e) => Ok(unnamed()),
		Err(e) => Err(Box::new(ge!(
			format!("{R}failed to read the tag of{D} {B}{path}{D}\n{e}"),
			h: format!("delete {B}{path}{D} and let the next run download it again")
		))),
	}
}

pub(crate) fn artist(tag: &Tag) -> String {
	joined(values(tag, ARTIST))
}

pub(crate) fn unnamed() -> Sort {
	of(&Tag::default())
}

fn nameless(e: &Error) -> bool {
	match &e.kind {
		ErrorKind::NoTag => true,
		ErrorKind::Io(e) => e.kind() == io::ErrorKind::NotFound,
		_ => false,
	}
}

fn of(tag: &Tag) -> Sort {
	let artist = joined(values(tag, ARTIST_SORT).or_else(|| values(tag, ARTIST))).to_lowercase();
	let title = folded(text(tag, TITLE_SORT).or_else(|| tag.title()));

	(artist.is_empty(), artist, title.is_empty(), title)
}

fn values<'a>(tag: &'a Tag, id: &str) -> Option<Vec<&'a str>> {
	let values = tag
		.get(id)
		.and_then(|frame| frame.content().text_values())?
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>();

	(!values.is_empty()).then_some(values)
}

fn joined(values: Option<Vec<&str>>) -> String {
	values.unwrap_or_default().join(ARTIST_SEPARATOR)
}

fn text<'a>(tag: &'a Tag, id: &str) -> Option<&'a str> {
	tag.get(id)
		.and_then(|frame| frame.content().text())
		.filter(|text| !text.trim().is_empty())
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
	fn every_credited_artist_sorts_under_one_name() {
		let mut tag = Tag::new();
		tag.set_text_values(ARTIST, ["菅野よう子", "Arnór Dan"]);
		tag.set_text_values(ARTIST_SORT, ["Kanno, Yōko", "Dan, Arnór"]);
		tag.set_title("von");

		assert_eq!(
			of(&tag),
			(
				false,
				"kanno, yōko & dan, arnór".to_string(),
				false,
				"von".to_string()
			)
		);
		assert_eq!(artist(&tag), "菅野よう子 & Arnór Dan");
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
	fn a_recording_carrying_a_blank_sort_name_sorts_by_the_name_it_has() {
		let mut tag = tag("Alan Walker", "", "Ignite", "");
		tag.set_text(ARTIST_SORT, "  ");
		tag.set_text(TITLE_SORT, "");

		assert_eq!(
			of(&tag),
			(
				false,
				"alan walker".to_string(),
				false,
				"ignite".to_string()
			)
		);
	}

	#[test]
	fn a_missing_or_untagged_recording_is_nameless_rather_than_unread() {
		assert!(nameless(&Error::new(ErrorKind::NoTag, "")));
		assert!(nameless(&Error::new(
			ErrorKind::Io(io::Error::from(io::ErrorKind::NotFound)),
			""
		)));
	}

	#[test]
	fn a_recording_whose_tag_cannot_be_read_is_not_nameless() {
		assert!(!nameless(&Error::new(ErrorKind::Parsing, "")));
		assert!(!nameless(&Error::new(
			ErrorKind::Io(io::Error::from(io::ErrorKind::PermissionDenied)),
			""
		)));
	}

	#[test]
	fn a_recording_that_is_not_downloaded_yet_sorts_last_without_complaint() {
		assert_eq!(sort(Source::nil()).ok(), Some(unnamed()));
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

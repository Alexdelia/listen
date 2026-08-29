use std::{collections::HashSet, path::Path};

use ansi::abbrev::{B, D, R};
use id3::{Tag, TagLike, Version, frame::ExtendedText};
use musicbrainz_rs::entity::recording::Recording;

use crate::{alias, library::tag::ARTIST_SEPARATOR, romaji};

const RECORDING_MBID: &str = "MusicBrainz Track Id";
const SUBTITLE: &str = "TIT3";
const TITLE_SORT: &str = "TSOT";
const ARTIST_SORT: &str = "TSOP";
const GENRE: &str = "TCON";

pub(super) fn write(path: &Path, recording: &Recording) -> Result<(), String> {
	let mut tag = Tag::read_from_path(path).unwrap_or_default();

	if !recording.title.is_empty() {
		tag.set_title(recording.title.as_str());
	}

	if let Some(artist_credit) = &recording.artist_credit
		&& !artist_credit.is_empty()
	{
		let artist = artist_credit
			.iter()
			.map(|ac| ac.artist.name.as_str())
			.collect::<Vec<_>>()
			.join(ARTIST_SEPARATOR);

		tag.set_artist(artist);
	}

	match title_sort(recording) {
		Some(sort) => tag.set_text(TITLE_SORT, sort),
		None => {
			tag.remove(TITLE_SORT);
		}
	}

	match artist_sort(recording) {
		Some(sort) => tag.set_text(ARTIST_SORT, sort),
		None => {
			tag.remove(ARTIST_SORT);
		}
	}

	match subtitle(recording) {
		Some(subtitle) => tag.set_text(SUBTITLE, subtitle),
		None => {
			tag.remove(SUBTITLE);
		}
	}

	let genre = genre(recording);
	if genre.is_empty() {
		tag.remove_genre();
	} else {
		tag.set_text_values(GENRE, genre);
	}

	tag.add_frame(ExtendedText {
		description: RECORDING_MBID.to_string(),
		value: recording.id.clone(),
	});

	tag.write_to_path(path, Version::default()).map_err(|e| {
		format!(
			"{R}failed to write metadata to {B}{path}{D}\n{e}",
			path = path.to_string_lossy(),
		)
	})
}

fn title_sort(recording: &Recording) -> Option<String> {
	let title = recording.title.trim();

	if romaji::latin(title) {
		return None;
	}

	romaji::of(title).or_else(|| romaji::of(reading(recording)?))
}

fn reading(recording: &Recording) -> Option<&str> {
	let alias = recording.aliases.as_deref()?;
	let title = recording.title.trim();

	alias
		.iter()
		.find(|a| a.name.trim() == title && romaji::kana(a.sort_name.trim()))
		.or_else(|| alias.iter().find(|a| romaji::kana(a.sort_name.trim())))
		.map(|a| a.sort_name.trim())
}

fn artist_sort(recording: &Recording) -> Option<String> {
	let artist_credit = recording.artist_credit.as_ref()?;
	let mut moved = false;

	let sort = artist_credit
		.iter()
		.map(|ac| {
			let name = ac.artist.name.trim();

			if romaji::latin(name) {
				return name.to_string();
			}

			let sort_name = ac.artist.sort_name.trim();

			if !sort_name.is_empty() && romaji::latin(sort_name) {
				moved = true;
				return sort_name.to_string();
			}

			romaji::of(name).map_or_else(
				|| name.to_string(),
				|romaji| {
					moved = true;
					romaji
				},
			)
		})
		.collect::<Vec<_>>()
		.join(ARTIST_SEPARATOR);

	moved.then_some(sort)
}

fn subtitle(recording: &Recording) -> Option<String> {
	let other_name = alias::other_name(recording.aliases.as_deref(), &recording.title);
	let comment = recording
		.disambiguation
		.as_deref()
		.map(str::trim)
		.filter(|comment| !comment.is_empty());

	match (other_name, comment) {
		(Some(other_name), Some(comment)) => Some(format!("{other_name} ({comment})")),
		(Some(other_name), None) => Some(other_name.to_string()),
		(None, Some(comment)) => Some(comment.to_string()),
		(None, None) => None,
	}
}

fn genre(recording: &Recording) -> Vec<&str> {
	let mut all = HashSet::new();

	if let Some(genres) = &recording.genres {
		all.extend(genres.iter().map(|g| g.name.as_str()));
	}
	if let Some(tags) = &recording.tags {
		all.extend(tags.iter().map(|t| t.name.as_str()));
	}

	let mut all = all.into_iter().collect::<Vec<_>>();
	all.sort_unstable();

	all
}

#[cfg(test)]
mod tests {
	use super::*;

	fn recording(title: &str, alias: &[&str], disambiguation: &str) -> Recording {
		let alias = alias
			.iter()
			.map(|name| format!(r#"{{"name": "{name}", "sort-name": "{name}", "primary": true}}"#))
			.collect::<Vec<_>>()
			.join(",");

		serde_json::from_str(&format!(
			r#"{{
				"id": "fbb4ccc1-2386-466e-a339-09594ac1bba6",
				"title": "{title}",
				"disambiguation": "{disambiguation}",
				"aliases": [{alias}]
			}}"#
		))
		.unwrap()
	}

	fn sorted(title: &str, reading: &[(&str, &str)], artist: &[(&str, &str)]) -> Recording {
		let alias = reading
			.iter()
			.map(|(name, sort_name)| {
				format!(r#"{{"name": "{name}", "sort-name": "{sort_name}", "primary": true}}"#)
			})
			.collect::<Vec<_>>()
			.join(",");

		let artist_credit = artist
			.iter()
			.map(|(name, sort_name)| {
				format!(
					r#"{{"name": "{name}", "artist": {{"id": "8f8b0f0f-0f0f-4f0f-8f0f-0f0f0f0f0f0f",
					"name": "{name}", "sort-name": "{sort_name}", "disambiguation": ""}}}}"#
				)
			})
			.collect::<Vec<_>>()
			.join(",");

		serde_json::from_str(&format!(
			r#"{{
				"id": "fbb4ccc1-2386-466e-a339-09594ac1bba6",
				"title": "{title}",
				"disambiguation": "",
				"aliases": [{alias}],
				"artist-credit": [{artist_credit}]
			}}"#
		))
		.unwrap()
	}

	#[test]
	fn the_other_name_alone_is_the_subtitle() {
		assert_eq!(
			subtitle(&recording(
				"忘れてやらない",
				&["忘れてやらない", "Never forget"],
				""
			)),
			Some("Never forget".to_string())
		);
	}

	#[test]
	fn the_disambiguation_alone_is_the_subtitle() {
		assert_eq!(
			subtitle(&recording("Secret base", &["Secret base"], "live")),
			Some("live".to_string())
		);
	}

	#[test]
	fn the_disambiguation_refines_the_other_name() {
		assert_eq!(
			subtitle(&recording("ひみつ基地", &["Secret base"], "live")),
			Some("Secret base (live)".to_string())
		);
	}

	#[test]
	fn a_recording_with_neither_has_no_subtitle() {
		assert_eq!(subtitle(&recording("Secret base", &[], "")), None);
	}
	#[test]
	fn a_kana_title_is_romanized_on_its_own() {
		assert_eq!(
			title_sort(&sorted("インフェルノ", &[], &[])),
			Some("infyeruno".to_string())
		);
	}

	#[test]
	fn a_kanji_title_is_romanized_off_the_kana_reading_its_alias_carries() {
		assert_eq!(
			title_sort(&sorted(
				"ひみつ基地",
				&[("ひみつ基地", "ひみつきち"), ("Secret base", "Secret base")],
				&[]
			)),
			Some("himitsukichi".to_string())
		);
	}

	#[test]
	fn a_kanji_title_whose_alias_only_translates_it_is_left_unsorted() {
		assert_eq!(
			title_sort(&sorted("勇者", &[("The Brave", "The Brave")], &[])),
			None
		);
	}

	#[test]
	fn a_title_already_in_latin_is_left_unsorted() {
		assert_eq!(title_sort(&sorted("Secret base", &[], &[])), None);
	}

	#[test]
	fn a_title_in_a_script_romaji_does_not_serve_is_left_unsorted() {
		assert_eq!(title_sort(&sorted("Кончится лето", &[], &[])), None);
		assert_eq!(title_sort(&sorted("우린 좀 달라", &[], &[])), None);
	}

	#[test]
	fn the_artist_sort_name_is_the_artist_sort() {
		assert_eq!(
			artist_sort(&sorted(
				"ひみつ基地",
				&[],
				&[("結束バンド", "Kessoku Band")]
			)),
			Some("Kessoku Band".to_string())
		);
	}

	#[test]
	fn an_artist_sort_name_that_never_left_its_script_is_romanized_instead() {
		assert_eq!(
			artist_sort(&sorted("オトノケ", &[], &[("ちか", "ちか")])),
			Some("chika".to_string())
		);
	}

	#[test]
	fn an_artist_neither_names_in_latin_keeps_the_name_it_has() {
		assert_eq!(
			artist_sort(&sorted("常世想兼神", &[], &[("匠眞", "匠眞")])),
			None
		);
	}

	#[test]
	fn every_credited_artist_is_sorted_together() {
		assert_eq!(
			artist_sort(&sorted(
				"von",
				&[],
				&[("菅野よう子", "Kanno, Yōko"), ("Arnór Dan", "Dan, Arnór")]
			)),
			Some("Kanno, Yōko & Arnór Dan".to_string())
		);
	}

	#[test]
	fn an_artist_already_in_latin_is_left_unsorted() {
		assert_eq!(
			artist_sort(&sorted("Ignite", &[], &[("Alan Walker", "Walker, Alan")])),
			None
		);
	}
}

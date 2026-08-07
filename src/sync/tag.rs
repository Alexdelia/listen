use std::{collections::HashSet, path::Path};

use ansi::abbrev::{B, D, R};
use id3::{Tag, TagLike, Version, frame::ExtendedText};
use musicbrainz_rs::entity::recording::Recording;

use crate::alias;

const RECORDING_MBID: &str = "MusicBrainz Track Id";
const SUBTITLE: &str = "TIT3";

pub fn write(path: &Path, recording: &Recording) -> Result<(), String> {
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
			.join(" & ");

		tag.set_artist(artist);
	}

	match subtitle(recording) {
		Some(subtitle) => tag.set_text(SUBTITLE, subtitle),
		None => {
			tag.remove(SUBTITLE);
		}
	}

	let genre = genre(recording);
	if !genre.is_empty() {
		tag.set_genre(genre);
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

fn genre(recording: &Recording) -> String {
	let mut all = HashSet::new();

	if let Some(genres) = &recording.genres {
		all.extend(genres.iter().map(|g| g.name.as_str()));
	}
	if let Some(tags) = &recording.tags {
		all.extend(tags.iter().map(|t| t.name.as_str()));
	}

	let mut all = all.into_iter().collect::<Vec<_>>();
	all.sort_unstable();

	all.join(" / ")
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
}

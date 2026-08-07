mod clipboard;

use std::{fs, path::Path};

use ansi::{
	DIM,
	abbrev::{B, CYA, D, R},
};
use hmerr::{ge, ioe};
use musicbrainz_rs::entity::{alias::Alias, artist_credit::ArtistCredit, recording::Recording};

use super::{duration, open, verify::Info};

const LIST_CLOSE: char = ']';
const UNKNOWN: &str = "?";

pub(super) fn recording(recording: &Recording, title: &str, length: i64) {
	println!(
		"{B}{title}{D}{aka}{comment} {DIM}-{D} {B}{artist}{D} {CYA}{dur}{D}",
		aka = aside(other_name(recording.aliases.as_deref(), title)),
		comment = aside(recording.disambiguation.as_deref()),
		artist = credit_line(recording.artist_credit.as_deref()),
		dur = duration::fmt(Some(length)),
	);
}

fn other_name<'a>(alias: Option<&'a [Alias]>, title: &str) -> Option<&'a str> {
	alias
		.into_iter()
		.flatten()
		.filter(|a| a.primary == Some(true))
		.map(|a| a.name.trim())
		.find(|name| *name != title.trim())
}

fn aside(s: Option<&str>) -> String {
	s.map(str::trim)
		.filter(|s| !s.is_empty())
		.map_or(String::default(), |s| format!(" {DIM}({s}){D}"))
}

fn credit_line(credit: Option<&[ArtistCredit]>) -> String {
	let mut out = String::new();

	for c in credit.into_iter().flatten() {
		out.push_str(&c.name);
		out.push_str(c.joinphrase.as_deref().unwrap_or_default());
	}

	let out = out.trim();

	if out.is_empty() {
		UNKNOWN.to_string()
	} else {
		out.to_string()
	}
}

pub(super) fn found(info: &Info, length: i64) {
	let delta_str = info.duration.map_or(String::default(), |dur| {
		let delta = dur - length;
		if delta == 0 {
			return String::default();
		}

		format!(" {R}{delta:+}{DIM}s{D}")
	});

	println!(
		"{B}{track}{D} {DIM}-{D} {B}{artist}{D} {CYA}{dur}{D}{delta_str}",
		track = info.track.as_deref().unwrap_or(UNKNOWN),
		artist = info.artist.as_deref().unwrap_or(UNKNOWN),
		dur = duration::fmt(info.duration),
	);
}

pub(super) fn entry(path: &Path, mbid: &str) -> hmerr::Result<()> {
	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	let Some(close) = content.rfind(LIST_CLOSE) else {
		return Err(ge!(format!(
			"{R}cannot append entry: {B}{path}{D} has no closing {B}{LIST_CLOSE}{D}",
			path = path.display(),
		))
		.into());
	};

	let entry = format!("\t(s: {mbid:?}, q: ?, playlist: []),\n");
	let content = format!(
		"{head}{entry}{tail}",
		head = &content[..close],
		tail = &content[close..]
	);

	fs::write(path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

pub(super) fn url(url: &str) {
	println!("{url}");
}

pub(super) fn musicbrainz(mbid: &str, url: &str) -> hmerr::Result<()> {
	clipboard::copy(url)?;
	open::open(&format!("https://musicbrainz.org/recording/{mbid}/edit"))?;
	println!("{B}musicbrainz{D} add free streaming relationship (copied)");

	Ok(())
}

#[cfg(test)]
mod tests {
	use musicbrainz_rs::entity::artist::Artist;

	use super::*;

	fn alias(name: &str, primary: bool) -> Alias {
		Alias {
			name: name.to_string(),
			primary: Some(primary),
			..Alias::default()
		}
	}

	#[test]
	fn the_primary_alias_that_is_not_the_title_is_the_other_name() {
		let alias = [alias("ひみつ基地", true), alias("Secret base", true)];

		assert_eq!(other_name(Some(&alias), "ひみつ基地"), Some("Secret base"));
	}

	#[test]
	fn a_non_primary_alias_is_never_the_other_name() {
		let alias = [alias("secret base", false)];

		assert_eq!(other_name(Some(&alias), "ひみつ基地"), None);
	}

	#[test]
	fn a_title_with_no_alias_stands_alone() {
		assert_eq!(other_name(None, "Secret base"), None);
	}

	#[test]
	fn an_aside_is_skipped_when_blank() {
		assert!(aside(None).is_empty());
		assert!(aside(Some("  ")).is_empty());
		assert!(aside(Some("live")).contains("(live)"));
	}

	#[test]
	fn a_credit_joins_every_artist_with_its_join_phrase() {
		let credit = |name: &str, joinphrase: &str| ArtistCredit {
			name: name.to_string(),
			joinphrase: Some(joinphrase.to_string()),
			artist: Artist::default(),
		};

		assert_eq!(
			credit_line(Some(&[credit("Kessoku Band", " feat. "), credit("Ao", "")])),
			"Kessoku Band feat. Ao"
		);
		assert_eq!(credit_line(None), UNKNOWN);
	}
}

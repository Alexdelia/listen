mod clipboard;

use std::{fs, path::Path};

use console::style;
use miette::{IntoDiagnostic, WrapErr, miette};
use musicbrainz_rs::entity::{artist_credit::ArtistCredit, recording::Recording};

use crate::{alias, format};

use super::{duration, open, verify::Info};

const LIST_CLOSE: char = ']';
const UNKNOWN: &str = "?";

pub(super) fn recording(recording: &Recording, title: &str, length: i64) {
	println!(
		"{}{aka}{comment} {} {} {}",
		style(&title).bold(),
		style("-").dim(),
		style(&credit_line(recording.artist_credit.as_deref())).bold(),
		style(&duration::fmt(Some(length))).cyan(),
		aka = aside(alias::other_name(recording.aliases.as_deref(), title)),
		comment = aside(recording.disambiguation.as_deref())
	);
}

fn aside(s: Option<&str>) -> String {
	s.map(str::trim)
		.filter(|s| !s.is_empty())
		.map_or_else(String::default, |s| {
			format!(" {}", style(format!("({s})")).dim())
		})
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
	let delta_str = info.duration.map_or_else(String::default, |dur| {
		let delta = dur - length;
		if delta == 0 {
			return String::default();
		}

		format!(" {:+}{}", style(&delta).red(), style("s").dim().red())
	});

	println!(
		"{} {} {} {}{delta_str}",
		style(&info.track.as_deref().unwrap_or(UNKNOWN)).bold(),
		style("-").dim(),
		style(&info.artist.as_deref().unwrap_or(UNKNOWN)).bold(),
		style(&duration::fmt(info.duration)).cyan()
	);
}

pub(super) fn entry(path: &Path, mbid: &str) -> miette::Result<()> {
	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	let Some(close) = content.rfind(LIST_CLOSE) else {
		return Err(miette!(
			"cannot append entry: {path} has no closing {LIST_CLOSE}",
			path = path.display()
		));
	};

	let entry = format!("\t(s: {mbid:?}, q: ?, playlist: []),\n");
	let content = format!(
		"{head}{entry}{tail}",
		head = &content[..close],
		tail = &content[close..]
	);

	fs::write(path, content)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

pub(super) fn url(url: &str) {
	println!("{}", format::url(url));
}

pub(super) fn musicbrainz(mbid: &str, url: &str) -> miette::Result<()> {
	clipboard::copy(url)?;
	open::open(&format!("https://musicbrainz.org/recording/{mbid}/edit"))?;
	println!(
		"{} add free streaming relationship (copied)",
		style("musicbrainz").bold()
	);

	Ok(())
}

#[cfg(test)]
mod tests {
	use musicbrainz_rs::entity::artist::Artist;

	use super::*;

	#[test]
	fn an_aside_is_skipped_when_blank() {
		assert_eq!(aside(None), "");
		assert_eq!(aside(Some("  ")), "");
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

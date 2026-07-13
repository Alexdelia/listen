mod clipboard;

use ansi::abbrev::{B, D};

use super::{duration, open, verify::Info};

pub(super) fn found(info: &Info, length: i64) {
	eprintln!(
		"{B}{track}{D} - {artist}, {dur} ({delta:+}s){album}",
		track = info.track.as_deref().unwrap_or("?"),
		artist = info.artist.as_deref().unwrap_or("?"),
		dur = duration::fmt(info.duration),
		delta = info.duration.unwrap_or(length) - length,
		album = info
			.album
			.as_deref()
			.map(|a| format!(", {a}"))
			.unwrap_or_default(),
	);
}

pub(super) fn entry(mbid: &str) {
	println!("(s: {mbid:?}, q: ?, playlist: [])");
}

pub(super) fn musicbrainz(mbid: &str, url: &str) -> hmerr::Result<()> {
	clipboard::copy(url)?;
	open::open(&format!("https://musicbrainz.org/recording/{mbid}/edit"))?;
	println!("{url}");
	eprintln!("{B}musicbrainz{D} add free streaming relationship (copied)");

	Ok(())
}

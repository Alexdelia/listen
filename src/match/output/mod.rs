mod clipboard;

use std::{fs, path::Path};

use ansi::{
	DIM,
	abbrev::{B, CYA, D, R},
};
use hmerr::{ge, ioe};

use super::{duration, open, verify::Info};

const LIST_CLOSE: char = ']';

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
		track = info.track.as_deref().unwrap_or("?"),
		artist = info.artist.as_deref().unwrap_or("?"),
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

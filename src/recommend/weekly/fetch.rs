use ansi::abbrev::{B, D, R};
use hmerr::ge;

use crate::{declaration::Source, meta_brainz};

const COUNT: usize = 100;

pub(super) fn created_for(username: &str) -> hmerr::Result<String> {
	body(
		&format!(
			"https://api.listenbrainz.org/1/user/{username}/playlists/createdfor?count={COUNT}"
		),
		&format!("{R}failed to fetch the playlists created for {B}{username}{D}"),
	)
}

pub(super) fn playlist(mbid: Source) -> hmerr::Result<String> {
	body(
		&format!("https://api.listenbrainz.org/1/playlist/{mbid}"),
		&format!("{R}failed to fetch playlist {B}{mbid}{D}"),
	)
}

fn body(url: &str, failure: &str) -> hmerr::Result<String> {
	meta_brainz::block_ready();

	let mut response = ureq::get(url)
		.call()
		.map_err(|e| ge!(format!("{failure}\n{e}")))?;

	response
		.body_mut()
		.read_to_string()
		.map_err(|e| ge!(format!("{failure}\n{e}")).into())
}

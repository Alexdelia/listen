use ansi::abbrev::{B, D, R};

use crate::{declaration::Source, listen_brainz};

const COUNT: usize = 1000;

pub(super) fn created_for(username: &str) -> hmerr::Result<String> {
	body(
		&format!("user/{username}/playlists/createdfor?count={COUNT}"),
		&format!("{R}failed to fetch the playlists created for {B}{username}{D}"),
	)
}

pub(super) fn playlist(mbid: Source) -> hmerr::Result<String> {
	body(
		&format!("playlist/{mbid}"),
		&format!("{R}failed to fetch playlist {B}{mbid}{D}"),
	)
}

fn body(path: &str, failure: &str) -> hmerr::Result<String> {
	Ok(listen_brainz::get(path, failure)?.body)
}

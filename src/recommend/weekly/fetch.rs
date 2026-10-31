use crate::{declaration::Source, listen_brainz};

const COUNT: usize = 1000;

pub(super) fn created_for(username: &str) -> miette::Result<String> {
	body(
		&format!("user/{username}/playlists/createdfor?count={COUNT}"),
		&format!("failed to fetch the playlists created for {username}"),
	)
}

pub(super) fn playlist(mbid: Source) -> miette::Result<String> {
	body(
		&format!("playlist/{mbid}"),
		&format!("failed to fetch playlist {mbid}"),
	)
}

fn body(path: &str, failure: &str) -> miette::Result<String> {
	Ok(listen_brainz::get(path, failure)?.body)
}

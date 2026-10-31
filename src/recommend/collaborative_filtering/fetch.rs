use miette::miette;
use ureq::http::StatusCode;

use crate::listen_brainz;

const PAGE: usize = 50;

pub(super) fn recording(username: &str, offset: usize) -> miette::Result<String> {
	let fetched = listen_brainz::get(
		&format!("cf/recommendation/user/{username}/recording?count={PAGE}&offset={offset}"),
		&format!("failed to fetch recommendation for {username}"),
	)?;

	if fetched.status == StatusCode::NO_CONTENT {
		return Err(miette!(
			help = "recommendations are computed periodically, come back later",
			"no recommendation computed for {username}"
		));
	}

	Ok(fetched.body)
}

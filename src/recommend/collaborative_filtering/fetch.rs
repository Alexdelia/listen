use ansi::abbrev::{B, D, R};
use hmerr::ge;
use ureq::http::StatusCode;

use crate::listen_brainz;

const PAGE: usize = 50;

pub(super) fn recording(username: &str, offset: usize) -> hmerr::Result<String> {
	let fetched = listen_brainz::get(
		&format!("cf/recommendation/user/{username}/recording?count={PAGE}&offset={offset}"),
		&format!("{R}failed to fetch recommendation for {B}{username}{D}"),
	)?;

	if fetched.status == StatusCode::NO_CONTENT {
		return Err(ge!(
			format!("{R}no recommendation computed for {B}{username}{D}"),
			h: "recommendations are computed periodically, come back later"
		)
		.into());
	}

	Ok(fetched.body)
}

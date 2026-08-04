use ansi::abbrev::{B, D, R};
use hmerr::ge;
use ureq::http::StatusCode;

use crate::meta_brainz;

const PAGE: usize = 50;

pub(super) fn recording(username: &str, offset: usize) -> hmerr::Result<String> {
	let url = format!(
		"https://api.listenbrainz.org/1/cf/recommendation/user/{username}/recording?count={PAGE}&offset={offset}"
	);

	meta_brainz::block_ready();

	let mut response = ureq::get(&url).call().map_err(|e| {
		ge!(format!(
			"{R}failed to fetch recommendation for {B}{username}{D}\n{e}"
		))
	})?;

	if response.status() == StatusCode::NO_CONTENT {
		return Err(ge!(
			format!("{R}no recommendation computed for {B}{username}{D}"),
			h: "recommendations are computed periodically, come back later"
		)
		.into());
	}

	response.body_mut().read_to_string().map_err(|e| {
		ge!(format!(
			"{R}failed to read recommendation for {B}{username}{D}\n{e}"
		))
		.into()
	})
}

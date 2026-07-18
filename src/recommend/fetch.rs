use ansi::abbrev::{B, D, R};
use hmerr::ge;
use serde::Deserialize;

use crate::{declaration::Source, meta_brainz};

const PAGE: usize = 20;
const NO_CONTENT: u16 = 204;

pub(super) struct Recommendation {
	pub mbid: Source,
	pub listened: bool,
}

pub(super) struct Page {
	pub recommendation: Vec<Recommendation>,
	pub fetched: usize,
	pub total: usize,
}

pub(super) fn page(username: &str, offset: usize) -> hmerr::Result<Page> {
	let url = format!(
		"https://api.listenbrainz.org/1/cf/recommendation/user/{username}/recording?count={PAGE}&offset={offset}"
	);

	meta_brainz::block_ready();

	let mut response = ureq::get(&url).call().map_err(|e| {
		ge!(format!(
			"{R}failed to fetch recommendation for {B}{username}{D}\n{e}"
		))
	})?;

	if response.status().as_u16() == NO_CONTENT {
		return Err(ge!(
			format!("{R}no recommendation computed for {B}{username}{D}"),
			h: "recommendations are computed periodically, come back later"
		)
		.into());
	}

	let body = response.body_mut().read_to_string().map_err(|e| {
		ge!(format!(
			"{R}failed to read recommendation for {B}{username}{D}\n{e}"
		))
	})?;

	let payload = serde_json::from_str::<Response>(&body)
		.map_err(|e| {
			ge!(format!(
				"{R}failed to parse recommendation for {B}{username}{D}\n{e}"
			))
		})?
		.payload;

	Ok(Page {
		recommendation: payload
			.mbids
			.into_iter()
			.map(|entry| Recommendation {
				mbid: entry.recording_mbid,
				listened: entry.latest_listened_at.is_some(),
			})
			.collect(),
		fetched: payload.count,
		total: payload.total_mbid_count,
	})
}

#[derive(Deserialize)]
struct Response {
	payload: Payload,
}

#[derive(Deserialize)]
struct Payload {
	count: usize,
	total_mbid_count: usize,
	mbids: Vec<Mbid>,
}

#[derive(Deserialize)]
struct Mbid {
	recording_mbid: Source,
	latest_listened_at: Option<String>,
}

use std::collections::HashMap;

use ansi::abbrev::{B, D, R, Y};
use hmerr::ge;
use serde::{Deserialize, Serialize};

use crate::entry::Source;

const RANGE: &str = "all_time";
const PAGE: usize = 1000;

pub(super) type ListenCount = HashMap<Source, Listen>;

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Listen {
	pub count: u32,
	pub track: String,
	pub artist: String,
}

pub(super) fn listen_count(username: &str) -> hmerr::Result<ListenCount> {
	let mut count = ListenCount::new();
	let mut fetched = 0;
	let mut total;

	loop {
		let payload = page(username, fetched)?;

		for recording in &payload.recordings {
			if let Some(mbid) = &recording.recording_mbid {
				count.insert(
					mbid.clone(),
					Listen {
						count: recording.listen_count,
						track: recording.track_name.clone(),
						artist: recording.artist_name.clone(),
					},
				);
			}
		}

		fetched += payload.count;
		total = payload.total_recording_count;
		if payload.count == 0 || fetched >= total {
			break;
		}
	}

	if total > fetched {
		eprintln!(
			"{Y}listenbrainz exposes only the top {fetched} recording of {total} listened, tail recording show as 0 listen{D}\n"
		);
	}

	Ok(count)
}

fn page(username: &str, offset: usize) -> hmerr::Result<Payload> {
	let url = format!(
		"https://api.listenbrainz.org/1/stats/user/{username}/recordings?range={RANGE}&count={PAGE}&offset={offset}"
	);

	let body = ureq::get(&url)
		.call()
		.map_err(|e| {
			ge!(format!(
				"{R}failed to fetch listen stats for {B}{username}{D}\n{e}"
			))
		})?
		.body_mut()
		.read_to_string()
		.map_err(|e| {
			ge!(format!(
				"{R}failed to read listen stats for {B}{username}{D}\n{e}"
			))
		})?;

	if body.trim().is_empty() {
		return Err(ge!(
			format!("{R}listenbrainz has no computed listen stats for {B}{username}{D}"),
			h: "stats are computed periodically, the user may have no listens yet"
		)
		.into());
	}

	let response: Response = serde_json::from_str(&body).map_err(|e| {
		ge!(format!(
			"{R}failed to parse listen stats for {B}{username}{D}\n{e}"
		))
	})?;

	Ok(response.payload)
}

#[derive(Deserialize)]
struct Response {
	payload: Payload,
}

#[derive(Deserialize)]
struct Payload {
	count: usize,
	total_recording_count: usize,
	recordings: Vec<StatRecording>,
}

#[derive(Deserialize)]
struct StatRecording {
	recording_mbid: Option<Source>,
	listen_count: u32,
	#[serde(default)]
	track_name: String,
	#[serde(default)]
	artist_name: String,
}

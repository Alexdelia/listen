use std::collections::HashMap;

use console::style;
use miette::miette;
use serde::{Deserialize, Serialize};

use crate::{declaration::Source, listen_brainz};

const RANGE: &str = "all_time";
const PAGE: usize = 1000;

pub(super) type ListenCount = HashMap<Source, Listen>;

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Listen {
	pub count: u32,
	pub track: String,
	pub artist: String,
}

pub(super) fn listen_count(username: &str) -> miette::Result<ListenCount> {
	let mut count = ListenCount::new();
	let mut fetched = 0;
	let mut total;

	loop {
		let payload = page(username, fetched)?;

		for recording in payload.recordings {
			if let Some(mbid) = recording.recording_mbid {
				count.insert(
					mbid,
					Listen {
						count: recording.listen_count,
						track: recording.track_name,
						artist: recording.artist_name,
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
			"{}\n",
			style(format!(
				"listenbrainz caps at top {fetched}/{total} recording: the tail shows 0 listen"
			))
			.yellow()
		);
	}

	Ok(count)
}

fn page(username: &str, offset: usize) -> miette::Result<Payload> {
	let body = listen_brainz::get(
		&format!("stats/user/{username}/recordings?range={RANGE}&count={PAGE}&offset={offset}"),
		&format!("failed to fetch listen stats for {username}"),
	)?
	.body;

	if body.trim().is_empty() {
		return Err(miette!(
			help = "stats are computed periodically, the user may have no listens yet",
			"listenbrainz has no computed listen stats for {username}"
		));
	}

	let response: Response = serde_json::from_str(&body)
		.map_err(|e| miette!("failed to parse listen stats for {username}\n{e}"))?;

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

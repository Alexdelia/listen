mod choose;
mod exploration;
mod fetch;
#[cfg(test)]
mod fixture;
mod jspf;
mod track;

use ansi::abbrev::{B, D, R, Y};
use hmerr::ge;

use crate::args::RecommendSource;

use super::{queue::Queue, selection};

use choose::choose;
use exploration::explorations;
use track::tracks;

pub(super) fn feed(username: &str, source: RecommendSource) -> hmerr::Result<Option<Queue>> {
	if !selection::weekly(source) {
		return Ok(None);
	}

	let fetched = fetch::created_for(username).and_then(|body| explorations(&body));
	let Some(found) = tolerated(source, fetched)? else {
		return Ok(None);
	};

	let chosen = choose(&found, source);
	if chosen.is_empty() {
		return missing(username, source);
	}

	let mut recommendation = Vec::new();
	for exploration in chosen {
		let fetched =
			fetch::playlist(exploration.mbid).and_then(|body| tracks(&body, exploration.week));

		if let Some(track) = tolerated(source, fetched)? {
			recommendation.extend(track);
		}
	}

	Ok(Some(Queue::new(recommendation)))
}

fn tolerated<T>(source: RecommendSource, fetched: hmerr::Result<T>) -> hmerr::Result<Option<T>> {
	match fetched {
		Ok(fetched) => Ok(Some(fetched)),
		Err(e) if selection::tolerates_missing_weekly(source) => {
			eprintln!("{Y}{e}{D}");
			Ok(None)
		}
		Err(e) => Err(e),
	}
}

fn missing(username: &str, source: RecommendSource) -> hmerr::Result<Option<Queue>> {
	if selection::tolerates_missing_weekly(source) {
		eprintln!("{Y}no weekly-exploration playlist for {B}{username}{D}");

		return Ok(None);
	}

	Err(ge!(
		format!("{R}no weekly-exploration playlist for {B}{username}{D}"),
		h: "listenbrainz keeps the two most recent, generated every monday"
	)
	.into())
}

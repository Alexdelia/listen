mod choose;
mod exploration;
mod fetch;
#[cfg(test)]
mod fixture;
mod jspf;
mod track;

use chrono::NaiveDate;
use console::style;
use miette::miette;

use crate::args::RecommendSource;

use super::{
	feed::{self, Fed},
	queue::Queue,
	selection,
};

use choose::choose;
use exploration::explorations;
use track::tracks;

pub(super) fn feed(username: &str, source: RecommendSource) -> miette::Result<Option<Fed>> {
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

	let week: Vec<NaiveDate> = chosen.iter().map(|exploration| exploration.week).collect();

	let mut recommendation = Vec::new();
	for exploration in chosen {
		let fetched =
			fetch::playlist(exploration.mbid).and_then(|body| tracks(&body, exploration.week));

		if let Some(track) = tolerated(source, fetched)? {
			recommendation.extend(track);
		}
	}

	Ok(Some(Fed {
		feed: Box::new(Queue::new(recommendation)),
		report: feed::Report::WeeklyExploration {
			username: username.to_string(),
			week,
		},
	}))
}

fn tolerated<T>(source: RecommendSource, fetched: miette::Result<T>) -> miette::Result<Option<T>> {
	match fetched {
		Ok(fetched) => Ok(Some(fetched)),
		Err(e) if selection::tolerates_missing_weekly(source) => {
			eprintln!("{e:?}");
			Ok(None)
		}
		Err(e) => Err(e),
	}
}

fn missing(username: &str, source: RecommendSource) -> miette::Result<Option<Fed>> {
	if selection::tolerates_missing_weekly(source) {
		eprintln!(
			"{}{}",
			style("no weekly-exploration playlist for ").yellow(),
			style(&username).bold().yellow()
		);

		return Ok(None);
	}

	Err(miette!(
		help = "listenbrainz keeps the two most recent, generated every monday",
		"no weekly-exploration playlist for {username}"
	))
}

use std::fmt::Display;

use ansi::{
	WHITE,
	abbrev::{B, D, F},
};
use chrono::{DateTime, NaiveDate, Utc};

use crate::{declaration::Source, format::DATE_FORMAT};

const COLLABORATIVE_FILTERING: &str = "collaborative-filtering";
const WEEKLY_EXPLORATION: &str = "weekly-exploration";
const LISTEN_BRAINZ: &str = "listenbrainz";
const ISLAND: &str = "island";

pub(super) struct Recommendation {
	pub mbid: Source,
	pub origin: Origin,
}

pub(super) enum Origin {
	CollaborativeFiltering {
		position: usize,
		score: f32,
		latest_listened_at: Option<DateTime<Utc>>,
	},
	WeeklyExploration {
		week: NaiveDate,
		position: usize,
	},
	ListenCount {
		listen: u64,
		user: u64,
		released: Option<NaiveDate>,
		position: usize,
	},
	Island {
		name: String,
		member: usize,
		score: f32,
		backer: u32,
		listener: u32,
		plays: u64,
		position: usize,
	},
}

impl Origin {
	pub(super) fn source(&self) -> String {
		match self {
			Self::CollaborativeFiltering { .. } => text(COLLABORATIVE_FILTERING),
			Self::WeeklyExploration { week, .. } => {
				precise(WEEKLY_EXPLORATION, week.format(DATE_FORMAT))
			}
			Self::ListenCount { .. } => text(LISTEN_BRAINZ),
			Self::Island { name, .. } => precise(ISLAND, name),
		}
	}

	pub(super) const fn position(&self) -> usize {
		match self {
			Self::CollaborativeFiltering { position, .. }
			| Self::WeeklyExploration { position, .. }
			| Self::ListenCount { position, .. }
			| Self::Island { position, .. } => *position,
		}
	}

	pub(super) const fn latest_listened_at(&self) -> Option<DateTime<Utc>> {
		match self {
			Self::CollaborativeFiltering {
				latest_listened_at, ..
			} => *latest_listened_at,
			Self::WeeklyExploration { .. } | Self::ListenCount { .. } | Self::Island { .. } => None,
		}
	}
}

fn text(origin: &str) -> String {
	format!("{B}{WHITE}{origin}{D}")
}

fn precise(origin: &str, precision: impl Display) -> String {
	format!("{origin} {F}{WHITE}{precision}{D}", origin = text(origin))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn collaborative_filtering(position: usize) -> Origin {
		Origin::CollaborativeFiltering {
			position,
			score: 1.0,
			latest_listened_at: None,
		}
	}

	fn listen_count(position: usize) -> Origin {
		Origin::ListenCount {
			listen: 1_259_231,
			user: 85_027,
			released: NaiveDate::from_ymd_opt(2010, 5, 24),
			position,
		}
	}

	fn weekly(position: usize) -> Origin {
		Origin::WeeklyExploration {
			week: NaiveDate::from_ymd_opt(2026, 7, 13).unwrap_or_default(),
			position,
		}
	}

	#[test]
	fn a_weekly_source_is_named_after_its_week() {
		assert_eq!(
			weekly(0).source(),
			format!("{B}{WHITE}weekly-exploration{D} {F}{WHITE}2026-07-13{D}")
		);
	}

	#[test]
	fn the_collaborative_filtering_source_has_no_date() {
		assert_eq!(
			collaborative_filtering(0).source(),
			format!("{B}{WHITE}collaborative-filtering{D}")
		);
	}

	#[test]
	fn a_listen_count_source_is_just_listenbrainz() {
		assert_eq!(
			listen_count(0).source(),
			format!("{B}{WHITE}listenbrainz{D}")
		);
	}

	#[test]
	fn the_position_comes_from_the_source_it_was_read_from() {
		assert_eq!(weekly(7).position(), 7);
		assert_eq!(collaborative_filtering(51).position(), 51);
		assert_eq!(listen_count(3).position(), 3);
	}
}

use chrono::{DateTime, NaiveDate, Utc};

use crate::declaration::Source;

const COLLABORATIVE_FILTERING: &str = "collaborative-filtering";
const WEEKLY_EXPLORATION: &str = "weekly-exploration";
const LISTEN_BRAINZ: &str = "listenbrainz";

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
		position: usize,
	},
}

impl Origin {
	pub(super) fn source(&self) -> String {
		match self {
			Self::CollaborativeFiltering { .. } => COLLABORATIVE_FILTERING.to_string(),
			Self::WeeklyExploration { week, .. } => format!("{WEEKLY_EXPLORATION} {week}"),
			Self::ListenCount { .. } => LISTEN_BRAINZ.to_string(),
		}
	}

	pub(super) fn position(&self) -> usize {
		match self {
			Self::CollaborativeFiltering { position, .. }
			| Self::WeeklyExploration { position, .. }
			| Self::ListenCount { position, .. } => *position,
		}
	}

	pub(super) fn latest_listened_at(&self) -> Option<DateTime<Utc>> {
		match self {
			Self::CollaborativeFiltering {
				latest_listened_at, ..
			} => *latest_listened_at,
			Self::WeeklyExploration { .. } | Self::ListenCount { .. } => None,
		}
	}
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
		assert_eq!(weekly(0).source(), "weekly-exploration 2026-07-13");
	}

	#[test]
	fn the_collaborative_filtering_source_has_no_date() {
		assert_eq!(
			collaborative_filtering(0).source(),
			"collaborative-filtering"
		);
	}

	#[test]
	fn a_listen_count_source_is_just_listenbrainz() {
		assert_eq!(listen_count(0).source(), "listenbrainz");
	}

	#[test]
	fn the_position_comes_from_the_source_it_was_read_from() {
		assert_eq!(weekly(7).position(), 7);
		assert_eq!(collaborative_filtering(51).position(), 51);
		assert_eq!(listen_count(3).position(), 3);
	}
}

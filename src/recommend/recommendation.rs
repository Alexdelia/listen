use std::fmt::Display;

use ansi::{
	WHITE,
	abbrev::{B, D, F},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use crate::{
	declaration::{Q, Source},
	format::DATE_FORMAT,
};

use super::labelled::Labelled;

const COLLABORATIVE_FILTERING: &str = "collaborative-filtering";
const WEEKLY_EXPLORATION: &str = "weekly-exploration";
const LISTEN_BRAINZ: &str = "listenbrainz";
const ISLAND: &str = "island";
const FORECAST: &str = "forecast";

#[derive(Serialize)]
pub(super) struct Recommendation {
	pub mbid: Source,
	pub origin: Origin,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
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
		stay: bool,
	},
	Forecast {
		expected: f32,
		raw: f32,
		support: u32,
		near: Vec<Near>,
		position: usize,
		worth: f32,
		redundancy: f32,
	},
}

#[derive(Serialize)]
pub(super) struct Near {
	#[serde(flatten)]
	pub recording: Labelled,
	pub q: Q,
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
			Self::Forecast { .. } => text(FORECAST),
		}
	}

	pub(super) const fn position(&self) -> usize {
		match self {
			Self::CollaborativeFiltering { position, .. }
			| Self::WeeklyExploration { position, .. }
			| Self::ListenCount { position, .. }
			| Self::Island { position, .. }
			| Self::Forecast { position, .. } => *position,
		}
	}

	pub(super) const fn latest_listened_at(&self) -> Option<DateTime<Utc>> {
		match self {
			Self::CollaborativeFiltering {
				latest_listened_at, ..
			} => *latest_listened_at,
			Self::WeeklyExploration { .. }
			| Self::ListenCount { .. }
			| Self::Island { .. }
			| Self::Forecast { .. } => None,
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

	#[test]
	fn the_forecast_source_is_just_forecast() {
		let forecast = Origin::Forecast {
			expected: 60.0,
			raw: 55.0,
			support: 5,
			near: Vec::new(),
			position: 4,
			worth: 60.0,
			redundancy: 0.0,
		};

		assert_eq!(forecast.source(), format!("{B}{WHITE}forecast{D}"));
		assert_eq!(forecast.position(), 4);
		assert_eq!(forecast.latest_listened_at(), None);
	}

	#[test]
	fn an_origin_serializes_under_its_source_name() {
		let island = Origin::Island {
			name: "touhou".to_string(),
			member: 30,
			score: 2.0,
			backer: 5,
			listener: 10,
			plays: 40,
			position: 0,
			stay: true,
		};
		let value = serde_json::to_value(&island).unwrap_or_default();

		assert_eq!(value["island"]["name"], "touhou");
		assert_eq!(value["island"]["stay"], true);
	}

	#[test]
	fn a_near_recording_serializes_as_its_mbid_label_and_q() {
		let near = Near {
			recording: Labelled(Source::from_bytes([7; 16])),
			q: 3,
		};
		let value = serde_json::to_value(&near).unwrap_or_default();

		assert_eq!(value["mbid"], "07070707-0707-0707-0707-070707070707");
		assert!(value["label"].is_null());
		assert_eq!(value["q"], 3);
	}
}

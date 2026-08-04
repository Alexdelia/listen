use chrono::{DateTime, NaiveDate, Utc};

use crate::declaration::Source;

pub(super) struct Recommendation {
	pub mbid: Source,
	pub origin: Origin,
}

pub(super) enum Origin {
	CollaborativeFiltering {
		score: f32,
		latest_listened_at: Option<DateTime<Utc>>,
	},
	WeeklyExploration {
		week: NaiveDate,
		position: usize,
	},
}

impl Origin {
	pub(super) fn latest_listened_at(&self) -> Option<DateTime<Utc>> {
		match self {
			Self::CollaborativeFiltering {
				latest_listened_at, ..
			} => *latest_listened_at,
			Self::WeeklyExploration { .. } => None,
		}
	}
}

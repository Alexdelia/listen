use ansi::abbrev::{CYA, D, GRE, Y};
use chrono::{DateTime, Months, Utc};

use super::super::recommendation::Origin;

const DATE_FORMAT: &str = "%Y-%m-%d";
const TIME_FORMAT: &str = "%H:%M";

pub(super) fn label(origin: &Origin) -> String {
	match origin {
		Origin::CollaborativeFiltering {
			score,
			latest_listened_at,
		} => format!(
			"{Y}{score:.3}{D}{last}",
			last = latest_listened_at
				.map(|at| format!(" {CYA}{at}{D}", at = listened(at)))
				.unwrap_or_default(),
		),
		Origin::WeeklyExploration { week, position } => {
			format!("{GRE}weekly-exploration {week} #{position}{D}")
		}
	}
}

fn listened(at: DateTime<Utc>) -> String {
	let recent = Utc::now()
		.checked_sub_months(Months::new(1))
		.is_some_and(|cutoff| at >= cutoff);

	let date_str = at.format(DATE_FORMAT).to_string();

	if recent {
		let time_str = at.format(TIME_FORMAT).to_string();
		format!("{date_str} {time_str}")
	} else {
		date_str
	}
}

#[cfg(test)]
mod tests {
	use chrono::{Months, NaiveDate, Utc};

	use super::*;

	fn week(year: i32, month: u32, day: u32) -> NaiveDate {
		NaiveDate::from_ymd_opt(year, month, day).unwrap_or_default()
	}

	#[test]
	fn collaborative_filtering_shows_the_score() {
		let shown = label(&Origin::CollaborativeFiltering {
			score: 0.432_1,
			latest_listened_at: None,
		});

		assert!(shown.contains("0.432"), "{shown}");
	}

	#[test]
	fn a_recent_listen_shows_date_and_time() {
		let at = Utc::now();
		let shown = label(&Origin::CollaborativeFiltering {
			score: 1.0,
			latest_listened_at: Some(at),
		});

		assert!(
			shown.contains(&at.format("%Y-%m-%d %H:%M").to_string()),
			"{shown}"
		);
	}

	#[test]
	fn an_old_listen_shows_only_the_date() {
		let at = Utc::now()
			.checked_sub_months(Months::new(2))
			.unwrap_or_default();
		let shown = label(&Origin::CollaborativeFiltering {
			score: 1.0,
			latest_listened_at: Some(at),
		});

		assert!(
			shown.contains(&at.format("%Y-%m-%d").to_string()),
			"{shown}"
		);
		assert!(!shown.contains(&at.format("%H:%M").to_string()), "{shown}");
	}

	#[test]
	fn weekly_exploration_shows_week_and_position() {
		let shown = label(&Origin::WeeklyExploration {
			week: week(2026, 7, 12),
			position: 3,
		});

		assert!(shown.contains("weekly-exploration"), "{shown}");
		assert!(shown.contains("2026-07-12"), "{shown}");
		assert!(shown.contains("#3"), "{shown}");
	}
}

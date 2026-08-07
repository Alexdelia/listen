use ansi::{
	WHITE,
	abbrev::{B, CYA, D, F, M, Y},
};
use chrono::{DateTime, Months, Utc};

use super::super::recommendation::{Origin, Recommendation};

const DATE_FORMAT: &str = "%Y-%m-%d";
const TIME_FORMAT: &str = "%H:%M";

pub(super) fn render(index: usize, recommendation: &Recommendation) -> String {
	format!(
		"{B}{M}{index}{D} {F}{WHITE}{source}{D} {F}{M}{position}{D}\n{B}{mbid}{D}{label}",
		source = recommendation.origin.source(),
		position = recommendation.origin.position(),
		mbid = recommendation.mbid,
		label = label(&recommendation.origin),
	)
}

fn label(origin: &Origin) -> String {
	match origin {
		Origin::CollaborativeFiltering {
			score,
			latest_listened_at,
			..
		} => format!(
			" {Y}{score:.3}{D}{last}",
			last = latest_listened_at
				.map(|at| format!(" {CYA}{at}{D}", at = listened(at)))
				.unwrap_or_default(),
		),
		Origin::WeeklyExploration { .. } => String::new(),
		Origin::ListenCount { listen, user, .. } => {
			format!(" {M}{listen} {F}listen{D} {CYA}{user} {F}user{D}")
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
	use crate::declaration::Source;

	fn collaborative_filtering(score: f32, latest_listened_at: Option<DateTime<Utc>>) -> Origin {
		Origin::CollaborativeFiltering {
			position: 0,
			score,
			latest_listened_at,
		}
	}

	fn recommendation(origin: Origin) -> Recommendation {
		Recommendation {
			mbid: Source::from_bytes([7; 16]),
			origin,
		}
	}

	#[test]
	fn the_index_the_source_and_its_position_sit_above_the_mbid() {
		let shown = render(
			3,
			&recommendation(Origin::WeeklyExploration {
				week: NaiveDate::from_ymd_opt(2026, 7, 12).unwrap_or_default(),
				position: 5,
			}),
		);
		let mut line = shown.lines();
		let head = line.next().unwrap_or_default();

		assert!(head.contains('3'), "{head}");
		assert!(head.contains("weekly-exploration 2026-07-12"), "{head}");
		assert!(head.ends_with(&format!("{F}{M}5{D}")), "{head}");

		let body = line.next().unwrap_or_default();

		assert!(
			body.contains("07070707-0707-0707-0707-070707070707"),
			"{body}"
		);
		assert_eq!(line.next(), None);
	}

	#[test]
	fn a_collaborative_filtering_score_stays_on_the_mbid_line() {
		let shown = render(0, &recommendation(collaborative_filtering(0.5, None)));
		let mut line = shown.lines();

		assert!(!line.next().unwrap_or_default().contains("0.500"));
		assert!(line.next().unwrap_or_default().contains("0.500"));
	}

	#[test]
	fn collaborative_filtering_shows_the_score() {
		let shown = label(&collaborative_filtering(0.432_1, None));

		assert!(shown.contains("0.432"), "{shown}");
	}

	#[test]
	fn a_recent_listen_shows_date_and_time() {
		let at = Utc::now();
		let shown = label(&collaborative_filtering(1.0, Some(at)));

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
		let shown = label(&collaborative_filtering(1.0, Some(at)));

		assert!(
			shown.contains(&at.format("%Y-%m-%d").to_string()),
			"{shown}"
		);
		assert!(!shown.contains(&at.format("%H:%M").to_string()), "{shown}");
	}

	#[test]
	fn a_listen_count_recommendation_shows_its_listen_and_user_count() {
		let shown = label(&Origin::ListenCount {
			listen: 1_259_231,
			user: 85_027,
			position: 0,
		});

		assert!(
			shown.contains(&format!("{M}1259231 {F}listen{D}")),
			"{shown}"
		);
		assert!(shown.contains(&format!("{CYA}85027 {F}user{D}")), "{shown}");
	}

	#[test]
	fn a_weekly_recommendation_adds_nothing_to_the_mbid() {
		let shown = label(&Origin::WeeklyExploration {
			week: NaiveDate::from_ymd_opt(2026, 7, 12).unwrap_or_default(),
			position: 3,
		});

		assert!(shown.is_empty(), "{shown}");
	}
}

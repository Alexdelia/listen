use chrono::{DateTime, Months, Utc};
use console::style;

use super::super::recommendation::{Origin, Recommendation};
use crate::{
	declaration::{Q, Source},
	format::{DATE_FORMAT, TIME_FORMAT, q_style},
	library,
};

pub(super) fn render(index: usize, recommendation: &Recommendation) -> String {
	format!(
		"{} {source} {}\n\
		{}{}{label}",
		style(&index).bold().magenta(),
		style(&recommendation.origin.position()).dim().magenta(),
		style("https://musicbrainz.org/recording/").cyan(),
		style(&recommendation.mbid).bold().cyan(),
		source = recommendation.origin.source(),
		label = label(&recommendation.origin)
	)
}

fn label(origin: &Origin) -> String {
	match origin {
		Origin::CollaborativeFiltering {
			score,
			latest_listened_at,
			..
		} => format!(
			" {:.3}{last}",
			style(&score).yellow(),
			last = latest_listened_at
				.map(|at| format!(" {}", style(&listened(at)).cyan()))
				.unwrap_or_default()
		),
		Origin::WeeklyExploration { .. } => String::new(),
		Origin::ListenCount {
			listen,
			user,
			released,
			..
		} => format!(
			" {} {} {} {}{released}",
			style(&listen).magenta(),
			style("listen").dim().magenta(),
			style(&user).cyan(),
			style("user").dim().cyan(),
			released = released
				.map(|date| format!(" {}", style(&date.format(DATE_FORMAT)).yellow()))
				.unwrap_or_default()
		),
		Origin::Island {
			member,
			score,
			backer,
			listener,
			plays,
			..
		} => format!(
			" {:.3} {} {} {} {} \
			{} {} {} {}",
			style(&score).yellow(),
			style(&plays).magenta(),
			style("play").dim().magenta(),
			style(&listener).cyan(),
			style("listener").dim().cyan(),
			style(&backer).blue(),
			style("backer").dim().blue(),
			style(&member).green(),
			style("seed").dim().green()
		),
		Origin::Forecast {
			expected,
			raw,
			support,
			near,
			..
		} => format!(
			" {:.0} {} {} {} {}{near}",
			style(&expected).yellow(),
			style("expected").dim().yellow(),
			style(format!("raw {raw:.0}")).dim(),
			style(&support).cyan(),
			style("support").dim().cyan(),
			near = near_line(
				&near
					.iter()
					.map(|near| (labelled(near.recording.0), near.q))
					.collect::<Vec<_>>()
			)
		),
	}
}

fn labelled(mbid: Source) -> String {
	let label = library::tag::label(mbid);

	if label.is_empty() {
		return mbid.to_string();
	}

	label
}

fn near_line(near: &[(String, Q)]) -> String {
	if near.is_empty() {
		return String::new();
	}

	let shown: Vec<String> = near
		.iter()
		.map(|(label, q)| format!("{label} {}", q_style(*q).apply_to(format!("q{q}"))))
		.collect();

	format!(
		"\n{} {}",
		style("near").dim(),
		shown.join(&format!("{} ", style(",").dim()))
	)
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
	use console::strip_ansi_codes;

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
		assert!(head.contains("weekly-exploration"), "{head}");
		assert!(head.contains("2026-07-12"), "{head}");
		assert!(strip_ansi_codes(head).ends_with('5'), "{head}");

		let body = line.next().unwrap_or_default();

		assert!(
			strip_ansi_codes(body)
				.contains("https://musicbrainz.org/recording/07070707-0707-0707-0707-070707070707"),
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
	fn an_island_recommendation_shows_its_play_before_its_listener_before_its_backer() {
		let shown = label(&Origin::Island {
			name: "touhou / speedcore".to_string(),
			member: 30,
			score: 1993.2,
			backer: 51,
			listener: 671,
			plays: 7083,
			position: 0,
			stay: true,
		});

		let at = |of: &str| {
			shown
				.find(of)
				.unwrap_or_else(|| unreachable!("{of} not shown in {shown}"))
		};

		let play = at("7083");
		let listener = at("671");
		let backer = at("51");

		assert!(play < listener && listener < backer, "{shown}");
		assert!(shown.contains("1993.200"), "{shown}");
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

	fn listen_count(released: Option<NaiveDate>) -> Origin {
		Origin::ListenCount {
			listen: 1_259_231,
			user: 85_027,
			released,
			position: 0,
		}
	}

	#[test]
	fn a_listen_count_recommendation_shows_its_listen_and_user_count() {
		let shown = strip_ansi_codes(&label(&listen_count(None))).into_owned();

		assert!(shown.contains("1259231 listen"), "{shown}");
		assert!(shown.contains("85027 user"), "{shown}");
	}

	#[test]
	fn a_dated_listen_count_recommendation_shows_its_release_date() {
		let shown = strip_ansi_codes(&label(&listen_count(NaiveDate::from_ymd_opt(2010, 5, 24))))
			.into_owned();

		assert!(shown.contains("2010-05-24"), "{shown}");
	}

	#[test]
	fn an_undated_listen_count_recommendation_ends_on_its_user_count() {
		let shown = strip_ansi_codes(&label(&listen_count(None))).into_owned();

		assert!(shown.ends_with("85027 user"), "{shown}");
	}

	#[test]
	fn a_weekly_recommendation_adds_nothing_to_the_mbid() {
		let shown = label(&Origin::WeeklyExploration {
			week: NaiveDate::from_ymd_opt(2026, 7, 12).unwrap_or_default(),
			position: 3,
		});

		assert!(shown.is_empty(), "{shown}");
	}

	fn forecast() -> Origin {
		Origin::Forecast {
			expected: 61.4,
			raw: 57.2,
			support: 23,
			near: Vec::new(),
			position: 0,
			worth: 61.4,
			redundancy: 0.0,
		}
	}

	#[test]
	fn a_forecast_recommendation_shows_its_expected_rating_raw_and_support() {
		let shown = label(&forecast());

		assert!(shown.contains("61"), "{shown}");
		assert!(shown.contains("raw 57"), "{shown}");
		assert!(shown.contains("23"), "{shown}");
		assert_eq!(shown.lines().count(), 1, "{shown}");
	}

	#[test]
	fn a_forecast_recommendation_lists_what_it_is_near_with_their_q() {
		let near = near_line(&[
			("Mela! - 緑黄色社会".to_string(), 3),
			("POP/STARS - K/DA".to_string(), 0),
		]);

		assert!(near.contains("Mela!") && near.contains("q3"), "{near}");
		assert!(near.contains("POP/STARS") && near.contains("q0"), "{near}");
	}
}

use ansi::{
	DIM,
	abbrev::{B, D},
};

use super::select::{MIN_SCORE, Suggestion};

pub(super) const LABEL: &str = "genre";

pub(super) fn line(suggestions: &[Suggestion]) -> String {
	if suggestions.is_empty() {
		return format!(
			"{B}{LABEL}{D}  {DIM}nothing above {threshold}%{D}",
			threshold = percent(MIN_SCORE)
		);
	}

	let listed = suggestions
		.iter()
		.map(|s| format!("{name} {DIM}{p}%{D}", name = s.name, p = percent(s.score)))
		.collect::<Vec<_>>()
		.join(&format!(" {DIM}·{D} "));

	format!("{B}{LABEL}{D}  {listed}")
}

fn percent(score: f32) -> String {
	format!("{:.0}", score.clamp(0.0, 1.0) * 100.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn suggestion(name: &str, score: f32) -> Suggestion {
		Suggestion {
			name: name.to_string(),
			score,
		}
	}

	#[test]
	fn a_score_is_shown_as_a_rounded_percent() {
		assert_eq!(percent(0.724), "72");
		assert_eq!(percent(0.725_1), "73");
		assert_eq!(percent(1.0), "100");
	}

	#[test]
	fn every_suggestion_is_listed_on_one_line_in_the_order_given() {
		let line = line(&[
			suggestion("vaporwave", 0.72),
			suggestion("future funk", 0.41),
		]);

		assert!(!line.contains('\n'));
		let vaporwave = line.find("vaporwave").unwrap();
		let future_funk = line.find("future funk").unwrap();
		assert!(vaporwave < future_funk);
		assert!(line.contains("72%"));
		assert!(line.contains("41%"));
	}

	#[test]
	fn nothing_to_suggest_says_what_the_threshold_was() {
		assert!(line(&[]).contains("nothing above 20%"));
	}
}

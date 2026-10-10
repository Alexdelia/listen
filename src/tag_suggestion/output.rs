use std::iter;

use ansi::{
	DIM,
	abbrev::{B, D, G},
};

use super::select::Suggestion;

pub(super) const LABEL: &str = "genre";
const CARRIED: &str = "on musicbrainz";

pub(super) fn rows(suggestions: &[Suggestion]) -> Vec<String> {
	if suggestions.is_empty() {
		return vec![format!("{B}{LABEL}{D}  {DIM}nothing to suggest{D}")];
	}

	let width = suggestions
		.iter()
		.map(|s| s.name.chars().count())
		.max()
		.unwrap_or_default();

	let listed = suggestions.iter().map(|s| {
		let carried = if s.carried {
			format!("  {G}{CARRIED}{D}")
		} else {
			String::new()
		};

		format!(
			"  {name:<width$} {DIM}{p:>3}%{D}{carried}",
			name = s.name,
			p = percent(s.score)
		)
	});

	iter::once(format!("{B}{LABEL}{D}")).chain(listed).collect()
}

fn percent(score: f32) -> String {
	format!("{:.0}", score.clamp(0.0, 1.0) * 100.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn suggestion(name: &str, score: f32, carried: bool) -> Suggestion {
		Suggestion {
			name: name.to_string(),
			score,
			carried,
		}
	}

	#[test]
	fn a_score_is_shown_as_a_rounded_percent() {
		assert_eq!(percent(0.724), "72");
		assert_eq!(percent(0.725_1), "73");
		assert_eq!(percent(1.0), "100");
	}

	#[test]
	fn every_suggestion_gets_its_own_row_under_the_label_in_the_order_given() {
		let rows = rows(&[
			suggestion("vaporwave", 0.72, false),
			suggestion("future funk", 0.41, false),
		]);

		assert_eq!(rows.len(), 3);
		assert!(rows[0].contains(LABEL));
		assert!(rows[1].contains("vaporwave") && rows[1].contains("72%"));
		assert!(rows[2].contains("future funk") && rows[2].contains("41%"));
	}

	#[test]
	fn names_are_padded_so_the_percents_line_up() {
		let rows = rows(&[
			suggestion("pop", 0.42, false),
			suggestion("pop rock", 0.18, false),
		]);

		assert_eq!(rows[1].find('%'), rows[2].find('%'));
	}

	#[test]
	fn only_a_genre_already_on_musicbrainz_says_so() {
		let rows = rows(&[
			suggestion("j-rock", 0.47, true),
			suggestion("pop", 0.42, false),
		]);

		assert!(rows[1].contains(CARRIED));
		assert!(!rows[2].contains(CARRIED));
	}

	#[test]
	fn nothing_to_suggest_is_said_on_one_row() {
		let rows = rows(&[]);

		assert_eq!(rows.len(), 1);
		assert!(rows[0].contains("nothing to suggest"));
	}
}

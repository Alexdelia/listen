use std::collections::{HashMap, HashSet};

const LIMIT: usize = 8;

#[derive(Debug, PartialEq)]
pub(super) struct Suggestion {
	pub(super) name: String,
	pub(super) score: f32,
	pub(super) carried: bool,
}

pub(super) fn select(genres: HashMap<String, f32>, carried: &HashSet<String>) -> Vec<Suggestion> {
	let mut suggestions = genres
		.into_iter()
		.map(|(name, score)| Suggestion {
			carried: carried.contains(&name),
			name,
			score,
		})
		.collect::<Vec<_>>();

	suggestions.sort_by(|a, b| {
		b.score
			.total_cmp(&a.score)
			.then_with(|| a.name.cmp(&b.name))
	});
	suggestions.truncate(LIMIT);

	suggestions
}

#[cfg(test)]
mod tests {
	use super::*;

	fn genres(scored: &[(&str, f32)]) -> HashMap<String, f32> {
		scored
			.iter()
			.map(|(name, score)| ((*name).to_string(), *score))
			.collect()
	}

	fn names(suggestions: &[Suggestion]) -> Vec<&str> {
		suggestions.iter().map(|s| s.name.as_str()).collect()
	}

	#[test]
	fn even_a_barely_scored_genre_is_suggested() {
		let selected = select(genres(&[("vaporwave", 0.01)]), &HashSet::new());

		assert_eq!(names(&selected), ["vaporwave"]);
	}

	#[test]
	fn suggestions_come_out_most_confident_first() {
		let selected = select(
			genres(&[("city pop", 0.3), ("vaporwave", 0.7), ("future funk", 0.4)]),
			&HashSet::new(),
		);

		assert_eq!(names(&selected), ["vaporwave", "future funk", "city pop"]);
	}

	#[test]
	fn no_more_than_eight_suggestions_are_kept() {
		let many = (0_u8..12)
			.map(|i| (format!("genre {i:02}"), 0.9 - f32::from(i) / 100.0))
			.collect::<HashMap<_, _>>();

		let selected = select(many, &HashSet::new());

		assert_eq!(selected.len(), 8);
		assert_eq!(selected[0].name, "genre 00");
		assert_eq!(selected[7].name, "genre 07");
	}

	#[test]
	fn a_genre_the_recording_already_carries_on_musicbrainz_is_kept_and_marked() {
		let carried = HashSet::from(["vaporwave".to_string()]);

		let selected = select(
			genres(&[("vaporwave", 0.9), ("future funk", 0.4)]),
			&carried,
		);

		assert_eq!(names(&selected), ["vaporwave", "future funk"]);
		assert!(selected[0].carried);
		assert!(!selected[1].carried);
	}
}

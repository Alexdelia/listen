use crate::args::RecommendSource;

use super::exploration::{CURRENT_WEEK, Exploration, LAST_WEEK};

pub(super) fn choose(found: &[Exploration], source: RecommendSource) -> Vec<&Exploration> {
	match source {
		RecommendSource::CollaborativeFiltering
		| RecommendSource::ListenBrainz
		| RecommendSource::Island => Vec::new(),
		RecommendSource::All | RecommendSource::WeeklyExploration => found.iter().rev().collect(),
		RecommendSource::WeeklyExplorationLastWeek => found.get(LAST_WEEK).into_iter().collect(),
		RecommendSource::WeeklyExplorationCurrentWeek => {
			found.get(CURRENT_WEEK).into_iter().collect()
		}
	}
}

#[cfg(test)]
mod tests {
	use chrono::NaiveDate;

	use super::{
		super::{
			exploration::explorations,
			fixture::{BARE_EXTENSION, CREATED_FOR, date},
		},
		*,
	};

	fn weeks(chosen: &[&Exploration]) -> Vec<NaiveDate> {
		chosen.iter().map(|e| e.week).collect()
	}

	#[test]
	fn collaborative_filtering_chooses_no_playlist() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert!(choose(&found, RecommendSource::CollaborativeFiltering).is_empty());
	}

	#[test]
	fn all_chooses_last_week_before_current_week() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert_eq!(
			weeks(&choose(&found, RecommendSource::All)),
			vec![date(2026, 7, 12), date(2026, 7, 28)]
		);
	}

	#[test]
	fn weekly_exploration_chooses_last_week_before_current_week() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert_eq!(
			weeks(&choose(&found, RecommendSource::WeeklyExploration)),
			vec![date(2026, 7, 12), date(2026, 7, 28)]
		);
	}

	#[test]
	fn last_week_chooses_the_second_newest_playlist() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert_eq!(
			weeks(&choose(&found, RecommendSource::WeeklyExplorationLastWeek)),
			vec![date(2026, 7, 12)]
		);
	}

	#[test]
	fn current_week_chooses_the_newest_playlist() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert_eq!(
			weeks(&choose(
				&found,
				RecommendSource::WeeklyExplorationCurrentWeek
			)),
			vec![date(2026, 7, 28)]
		);
	}

	#[test]
	fn a_single_playlist_leaves_last_week_empty() {
		let found = explorations(BARE_EXTENSION).unwrap_or_default();

		assert!(choose(&found, RecommendSource::WeeklyExplorationLastWeek).is_empty());
	}
}

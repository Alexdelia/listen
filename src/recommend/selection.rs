use crate::args::RecommendSource;

pub(super) fn weekly(source: RecommendSource) -> bool {
	!matches!(source, RecommendSource::CollaborativeFiltering)
}

pub(super) fn collaborative_filtering(source: RecommendSource) -> bool {
	matches!(
		source,
		RecommendSource::All | RecommendSource::CollaborativeFiltering
	)
}

pub(super) fn tolerates_missing_weekly(source: RecommendSource) -> bool {
	matches!(
		source,
		RecommendSource::All | RecommendSource::WeeklyExploration
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_walks_both_sources() {
		assert!(weekly(RecommendSource::All));
		assert!(collaborative_filtering(RecommendSource::All));
	}

	#[test]
	fn a_weekly_source_leaves_collaborative_filtering_out() {
		for source in [
			RecommendSource::WeeklyExploration,
			RecommendSource::WeeklyExplorationLastWeek,
			RecommendSource::WeeklyExplorationCurrentWeek,
		] {
			assert!(weekly(source));
			assert!(!collaborative_filtering(source));
		}
	}

	#[test]
	fn collaborative_filtering_asks_for_no_playlist() {
		assert!(!weekly(RecommendSource::CollaborativeFiltering));
		assert!(collaborative_filtering(
			RecommendSource::CollaborativeFiltering
		));
	}

	#[test]
	fn an_explicit_week_makes_a_missing_playlist_fatal() {
		assert!(!tolerates_missing_weekly(
			RecommendSource::WeeklyExplorationLastWeek
		));
		assert!(!tolerates_missing_weekly(
			RecommendSource::WeeklyExplorationCurrentWeek
		));
	}

	#[test]
	fn a_broad_source_tolerates_a_missing_playlist() {
		assert!(tolerates_missing_weekly(RecommendSource::All));
		assert!(tolerates_missing_weekly(RecommendSource::WeeklyExploration));
	}
}

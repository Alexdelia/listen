use ansi::abbrev::{B, D, R};
use hmerr::ge;

use clap::ValueEnum;

use crate::args::RecommendSource;

use super::target::Target;

pub(super) fn weekly(source: RecommendSource) -> bool {
	matches!(
		source,
		RecommendSource::All
			| RecommendSource::WeeklyExploration
			| RecommendSource::WeeklyExplorationLastWeek
			| RecommendSource::WeeklyExplorationCurrentWeek
	)
}

pub(super) fn collaborative_filtering(source: RecommendSource) -> bool {
	matches!(
		source,
		RecommendSource::All | RecommendSource::CollaborativeFiltering
	)
}

pub(super) fn listen_count(source: RecommendSource) -> bool {
	matches!(source, RecommendSource::All | RecommendSource::ListenBrainz)
}

pub(super) fn tolerates_missing_weekly(source: RecommendSource) -> bool {
	matches!(
		source,
		RecommendSource::All | RecommendSource::WeeklyExploration
	)
}

pub(super) fn ensure(source: RecommendSource, target: &Target) -> hmerr::Result<()> {
	let (fits, need) = match target {
		Target::Username(_) => (
			weekly(source) || collaborative_filtering(source),
			"an artist mbid, not a username",
		),
		Target::Artist(_) => (listen_count(source), "a username, not an mbid"),
	};

	if fits {
		return Ok(());
	}

	Err(ge!(format!(
		"{R}source {B}{source}{D}{R} needs {need}{D}",
		source = name(source)
	))
	.into())
}

fn name(source: RecommendSource) -> String {
	source
		.to_possible_value()
		.map_or_else(String::new, |value| value.get_name().to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::declaration::Source;

	fn artist() -> Target {
		Target::Artist(Source::from_bytes([1; 16]))
	}

	fn username() -> Target {
		Target::Username("alexdelia".to_string())
	}

	const USERNAME_SOURCE: [RecommendSource; 4] = [
		RecommendSource::CollaborativeFiltering,
		RecommendSource::WeeklyExploration,
		RecommendSource::WeeklyExplorationLastWeek,
		RecommendSource::WeeklyExplorationCurrentWeek,
	];

	#[test]
	fn all_walks_every_source_of_a_username() {
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
	fn listenbrainz_is_the_only_listen_count_source() {
		assert!(listen_count(RecommendSource::ListenBrainz));
		assert!(listen_count(RecommendSource::All));

		for source in USERNAME_SOURCE {
			assert!(!listen_count(source));
		}
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

	#[test]
	fn all_fits_every_target() {
		assert!(ensure(RecommendSource::All, &username()).is_ok());
		assert!(ensure(RecommendSource::All, &artist()).is_ok());
	}

	#[test]
	fn a_username_source_needs_a_username() {
		for source in USERNAME_SOURCE {
			assert!(ensure(source, &username()).is_ok());
			assert!(ensure(source, &artist()).is_err());
		}
	}

	#[test]
	fn listenbrainz_needs_an_mbid() {
		assert!(ensure(RecommendSource::ListenBrainz, &artist()).is_ok());
		assert!(ensure(RecommendSource::ListenBrainz, &username()).is_err());
	}

	fn refusal(source: RecommendSource, target: &Target) -> String {
		ensure(source, target)
			.err()
			.map(|e| e.to_string())
			.unwrap_or_default()
	}

	#[test]
	fn a_username_source_asked_of_an_mbid_says_it_wants_a_username() {
		let said = refusal(RecommendSource::WeeklyExploration, &artist());

		assert!(said.contains("weekly-exploration"), "{said}");
		assert!(said.contains("needs a username"), "{said}");
	}

	#[test]
	fn listenbrainz_asked_of_a_username_says_it_wants_an_mbid() {
		let said = refusal(RecommendSource::ListenBrainz, &username());

		assert!(said.contains("listenbrainz"), "{said}");
		assert!(said.contains("needs an artist mbid"), "{said}");
	}
}

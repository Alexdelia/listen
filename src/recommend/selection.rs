use ansi::abbrev::{B, D, R};
use hmerr::ge;

use clap::ValueEnum;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use super::target::Target;

pub(super) fn island(source: RecommendSource) -> bool {
	matches!(source, RecommendSource::Island)
}

pub(super) fn ensure_island(
	sort: RecommendSort,
	target: Option<&str>,
	arg: &IslandArg,
) -> hmerr::Result<()> {
	if let Some(target) = target {
		return Err(ge!(
			format!("{R}source {B}island{D}{R} takes no target, got {B}{target}{D}"),
			h: "it reads the declaration and the local index, never a listenbrainz account"
		)
		.into());
	}

	if sort != RecommendSort::Popularity {
		return Err(ge!(format!(
			"{R}sort {B}{sort}{D}{R} needs an artist mbid, not source {B}island{D}",
			sort = name(&sort)
		))
		.into());
	}

	if !arg.seed.is_empty() && arg.island.is_some() {
		return Err(ge!(
			format!("{R}{B}--island{D}{R} cannot pin an island built by {B}--seed{D}"),
			h: "--seed already says which recordings the island is made of"
		)
		.into());
	}

	if !arg.genre.is_empty() && arg.island.is_some() {
		return Err(ge!(
			format!("{R}{B}--island{D}{R} cannot pin an island built by {B}--genre{D}"),
			h: "--genre already says which recordings the island is made of"
		)
		.into());
	}

	if arg.alpha.is_some_and(|alpha| alpha < 0.0) {
		return Err(ge!(
			format!("{R}{B}--alpha{D}{R} damps popularity, so it cannot be negative{D}"),
			h: "0 leaves popularity alone, 0.6 is the discovery setting"
		)
		.into());
	}

	if arg.resolution.is_some_and(|resolution| resolution <= 0.0) {
		return Err(ge!(format!("{R}{B}--resolution{D}{R} has to be above zero{D}")).into());
	}

	Ok(())
}

pub(super) fn ensure_no_island_arg(source: RecommendSource, arg: &IslandArg) -> hmerr::Result<()> {
	let unusable = [
		("--island", arg.island.is_some()),
		("--ask", arg.ask),
		("--seed", !arg.seed.is_empty()),
		("--genre", !arg.genre.is_empty()),
		("--alpha", arg.alpha.is_some()),
		("--resolution", arg.resolution.is_some()),
	];

	let Some((flag, _)) = unusable.iter().find(|(_, given)| *given) else {
		return Ok(());
	};

	Err(ge!(
		format!(
			"{R}{B}{flag}{D}{R} needs source {B}island{D}{R}, not {B}{source}{D}",
			source = name(&source)
		),
		h: format!("run with {B}--source island{D}")
	)
	.into())
}

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

pub(super) fn ensure(
	source: RecommendSource,
	sort: RecommendSort,
	target: &Target,
) -> hmerr::Result<()> {
	let (fits, need) = match target {
		Target::Username(_) => (
			weekly(source) || collaborative_filtering(source),
			"an artist mbid, not a username",
		),
		Target::Artist(_) => (listen_count(source), "a username, not an mbid"),
	};

	if !fits {
		return Err(ge!(format!(
			"{R}source {B}{source}{D}{R} needs {need}{D}",
			source = name(&source)
		))
		.into());
	}

	if sort != RecommendSort::Popularity && !matches!(target, Target::Artist(_)) {
		return Err(ge!(
			format!(
				"{R}sort {B}{sort}{D}{R} needs an artist mbid, not a username{D}",
				sort = name(&sort)
			),
			h: "only the recordings of an artist carry a release date to sort on"
		)
		.into());
	}

	Ok(())
}

fn name<T: ValueEnum>(value: &T) -> String {
	value
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

	fn by_popularity(source: RecommendSource, target: &Target) -> hmerr::Result<()> {
		ensure(source, RecommendSort::Popularity, target)
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
		assert!(by_popularity(RecommendSource::All, &username()).is_ok());
		assert!(by_popularity(RecommendSource::All, &artist()).is_ok());
	}

	#[test]
	fn a_username_source_needs_a_username() {
		for source in USERNAME_SOURCE {
			assert!(by_popularity(source, &username()).is_ok());
			assert!(by_popularity(source, &artist()).is_err());
		}
	}

	#[test]
	fn listenbrainz_needs_an_mbid() {
		assert!(by_popularity(RecommendSource::ListenBrainz, &artist()).is_ok());
		assert!(by_popularity(RecommendSource::ListenBrainz, &username()).is_err());
	}

	fn refusal(source: RecommendSource, target: &Target) -> String {
		by_popularity(source, target)
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

	#[test]
	fn newest_fits_an_artist() {
		assert!(
			ensure(
				RecommendSource::ListenBrainz,
				RecommendSort::Newest,
				&artist()
			)
			.is_ok()
		);
		assert!(ensure(RecommendSource::All, RecommendSort::Newest, &artist()).is_ok());
	}

	#[test]
	fn newest_asked_of_a_username_says_it_wants_an_mbid() {
		let said = ensure(
			RecommendSource::CollaborativeFiltering,
			RecommendSort::Newest,
			&username(),
		)
		.err()
		.map(|e| e.to_string())
		.unwrap_or_default();

		assert!(said.contains("newest"), "{said}");
		assert!(said.contains("needs an artist mbid"), "{said}");
	}

	#[test]
	fn popularity_fits_a_username() {
		for source in USERNAME_SOURCE {
			assert!(ensure(source, RecommendSort::Popularity, &username()).is_ok());
		}
	}
}

use ansi::abbrev::{B, D, R};
use hmerr::ge;

use clap::ValueEnum;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use super::target::Target;

pub(super) fn island(source: RecommendSource) -> bool {
	matches!(source, RecommendSource::All | RecommendSource::Island)
}

pub(super) fn island_only(source: RecommendSource) -> bool {
	matches!(source, RecommendSource::Island)
}

pub(super) fn ensure_island_target(sort: RecommendSort, target: Option<&str>) -> hmerr::Result<()> {
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

	Ok(())
}

pub(super) fn ensure_arg(source: RecommendSource, arg: &IslandArg) -> hmerr::Result<()> {
	if island(source) {
		return ensure_island_arg(arg);
	}

	ensure_no_island_arg(source, arg)
}

fn ensure_island_arg(arg: &IslandArg) -> hmerr::Result<()> {
	if let Some(built_by) = built_by(arg) {
		ensure_built_island_arg(arg, built_by)?;
	}

	if arg.popularity_damp.is_some_and(|damp| damp < 0.0) {
		return Err(ge!(
			format!("{R}{B}--popularity-damp{D}{R} cannot be negative{D}"),
			h: "0 leaves popularity alone, 0.6 is the discovery setting, higher digs further"
		)
		.into());
	}

	if arg
		.granularity
		.is_some_and(|granularity| granularity <= 0.0)
	{
		return Err(ge!(format!("{R}{B}--granularity{D}{R} has to be above zero{D}")).into());
	}

	Ok(())
}

fn built_by(arg: &IslandArg) -> Option<&'static str> {
	if !arg.seed.is_empty() {
		return Some("--seed");
	}

	if !arg.genre.is_empty() {
		return Some("--genre");
	}

	None
}

fn ensure_built_island_arg(arg: &IslandArg, built_by: &str) -> hmerr::Result<()> {
	if arg.island.is_some() {
		return Err(ge!(
			format!("{R}{B}--island{D}{R} cannot pin an island built by {B}{built_by}{D}"),
			h: format!("{built_by} already says which recordings the island is made of")
		)
		.into());
	}

	if arg.granularity.is_some() {
		return Err(ge!(
			format!("{R}{B}--granularity{D}{R} cannot split an island built by {B}{built_by}{D}"),
			h: "granularity only tunes the islands detected out of the whole declaration"
		)
		.into());
	}

	Ok(())
}

fn ensure_no_island_arg(source: RecommendSource, arg: &IslandArg) -> hmerr::Result<()> {
	let unusable = [
		("--island", arg.island.is_some()),
		("--ask", arg.ask),
		("--seed", !arg.seed.is_empty()),
		("--genre", !arg.genre.is_empty()),
		("--popularity-damp", arg.popularity_damp.is_some()),
		("--granularity", arg.granularity.is_some()),
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

	fn no_arg() -> IslandArg {
		IslandArg {
			popularity_damp: None,
			granularity: None,
			island: None,
			ask: false,
			seed: Vec::new(),
			genre: Vec::new(),
		}
	}

	#[test]
	fn an_island_flag_needs_a_source_that_reaches_the_index() {
		for source in USERNAME_SOURCE {
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						ask: true,
						..no_arg()
					}
				)
				.is_err()
			);
		}
	}

	#[test]
	fn all_takes_the_island_flags_it_can_hand_to_the_index() {
		assert!(
			ensure_arg(
				RecommendSource::All,
				&IslandArg {
					ask: true,
					granularity: Some(1.5),
					..no_arg()
				}
			)
			.is_ok()
		);
	}

	#[test]
	fn a_negative_damp_is_refused_by_every_island_source() {
		for source in [RecommendSource::All, RecommendSource::Island] {
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						popularity_damp: Some(-1.0),
						..no_arg()
					}
				)
				.is_err()
			);
		}
	}

	#[test]
	fn a_granularity_of_zero_is_refused_by_every_island_source() {
		for source in [RecommendSource::All, RecommendSource::Island] {
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						granularity: Some(0.0),
						..no_arg()
					}
				)
				.is_err()
			);
		}
	}

	#[test]
	fn a_pinned_island_cannot_also_be_built() {
		for source in [RecommendSource::All, RecommendSource::Island] {
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						island: Some("touhou".to_string()),
						seed: vec![Source::from_bytes([2; 16])],
						..no_arg()
					}
				)
				.is_err()
			);
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						island: Some("touhou".to_string()),
						genre: vec!["eurobeat".to_string()],
						..no_arg()
					}
				)
				.is_err()
			);
		}
	}

	#[test]
	fn a_built_island_cannot_also_be_split() {
		for source in [RecommendSource::All, RecommendSource::Island] {
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						granularity: Some(1.5),
						seed: vec![Source::from_bytes([2; 16])],
						..no_arg()
					}
				)
				.is_err()
			);
			assert!(
				ensure_arg(
					source,
					&IslandArg {
						granularity: Some(1.5),
						genre: vec!["eurobeat".to_string()],
						..no_arg()
					}
				)
				.is_err()
			);
		}
	}

	#[test]
	fn a_pinned_island_is_still_detected_at_the_asked_granularity() {
		assert!(
			ensure_arg(
				RecommendSource::Island,
				&IslandArg {
					granularity: Some(1.5),
					island: Some("touhou".to_string()),
					..no_arg()
				}
			)
			.is_ok()
		);
	}

	#[test]
	fn island_takes_no_target() {
		assert!(ensure_island_target(RecommendSort::Popularity, None).is_ok());
		assert!(ensure_island_target(RecommendSort::Popularity, Some("alexdelia")).is_err());
		assert!(ensure_island_target(RecommendSort::Newest, None).is_err());
	}
}

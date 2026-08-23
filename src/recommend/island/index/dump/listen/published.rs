use super::super::{rsync, stamp};

pub(super) const MODULE: &str = "listenbrainz/fullexport";
pub(crate) const PREFIX: &str = "listenbrainz-dump-";
const SUFFIX: &str = "-full";

pub(crate) fn newer_than(baseline: &str) -> hmerr::Result<Option<String>> {
	let built = stamp::reach(baseline)?;

	Ok(newest()?.filter(|name| reaches_past(name, built)))
}

pub(super) fn newest() -> hmerr::Result<Option<String>> {
	let published = rsync::list(&format!("{host}/{MODULE}/", host = rsync::HOST))?;

	Ok(newest_of(published.into_iter().map(|entry| entry.name)))
}

fn reaches_past(name: &str, built: u64) -> bool {
	reaches(name).is_some_and(|reach| reach > built)
}

fn newest_of(name: impl Iterator<Item = String>) -> Option<String> {
	name.filter_map(|name| Some((number(&name)?, name)))
		.max()
		.map(|(_, name)| name)
}

fn number(name: &str) -> Option<u32> {
	stamp::published(name, PREFIX, SUFFIX).map(|published| published.number)
}

fn reaches(name: &str) -> Option<u64> {
	stamp::published(name, PREFIX, SUFFIX).map(|published| published.reach)
}

#[cfg(test)]
mod tests {
	use super::*;

	const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";
	const BASELINE: &str = "listenbrainz-dump-2593-20260712-000004-full";

	fn repairs(name: &str, baseline: &str) -> bool {
		reaches_past(name, stamp::reach(baseline).unwrap_or_default())
	}

	fn published(name: &[&str]) -> Option<String> {
		newest_of(name.iter().map(|name| (*name).to_string()))
	}

	#[test]
	fn the_dump_number_is_read_out_of_the_published_name() {
		assert_eq!(
			number("listenbrainz-dump-2593-20260712-000004-full"),
			Some(2593)
		);
		assert_eq!(
			number("listenbrainz-dump-2593-20260712-000004-incremental"),
			None
		);
		assert_eq!(number("LATEST"), None);
	}

	#[test]
	fn the_newest_published_dump_is_the_highest_numbered_one() {
		assert_eq!(
			published(&[
				"listenbrainz-dump-2592-20260705-000003-full",
				"listenbrainz-dump-2593-20260712-000004-full",
			]),
			Some("listenbrainz-dump-2593-20260712-000004-full".to_string())
		);
	}

	#[test]
	fn a_wider_dump_number_is_still_the_newer_one() {
		assert_eq!(
			published(&[
				"listenbrainz-dump-1000-20340101-000001-full",
				"listenbrainz-dump-999-20330101-000001-full",
			]),
			Some("listenbrainz-dump-1000-20340101-000001-full".to_string())
		);
	}

	#[test]
	fn a_full_dump_published_past_the_baseline_is_the_one_that_repairs_a_gap() {
		assert!(
			repairs("listenbrainz-dump-2600-20260901-000003-full", BUILT),
			"what the index absorbed its way to is no reason to leave a hole unrepaired"
		);
	}

	#[test]
	fn the_dump_the_index_was_already_built_from_repairs_nothing() {
		assert!(!repairs(BASELINE, BUILT));
	}

	#[test]
	fn a_full_dump_older_than_the_baseline_repairs_nothing() {
		assert!(!repairs(
			"listenbrainz-dump-2592-20260705-000003-full",
			BUILT
		));
	}

	#[test]
	fn nothing_published_is_nothing_to_fetch() {
		assert_eq!(published(&["LATEST", "index.html"]), None);
	}
}

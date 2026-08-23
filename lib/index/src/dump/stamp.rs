use ansi::abbrev::{B, D, R};
use hmerr::{GenericError, ge};

const SUB_SECOND: char = '.';
const FIELD: char = '-';
const DIGIT: usize = 14;

pub(super) struct Published {
	pub number: u32,
	pub reach: u64,
}

pub(super) fn reach(timestamp: &str) -> hmerr::Result<u64> {
	key(timestamp).ok_or_else(|| unreadable(timestamp).into())
}

fn unreadable(timestamp: &str) -> GenericError {
	ge!(
		format!("{R}cannot read the timestamp {B}{timestamp}{D}"),
		h: "delete the index to build it again from a dump"
	)
}

fn key(timestamp: &str) -> Option<u64> {
	let digit: String = timestamp
		.split(SUB_SECOND)
		.next()?
		.chars()
		.filter(char::is_ascii_digit)
		.take(DIGIT)
		.collect();

	if digit.len() != DIGIT {
		return None;
	}

	digit.parse().ok()
}

pub(super) fn published(name: &str, prefix: &str, suffix: &str) -> Option<Published> {
	let mut field = name
		.strip_prefix(prefix)?
		.strip_suffix(suffix)?
		.split(FIELD);

	let number = field.next()?.parse().ok()?;
	let date = field.next()?;
	let time = field.next()?;

	Some(Published {
		number,
		reach: format!("{date}{time}").parse().ok()?,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	const PREFIX: &str = "listenbrainz-dump-";
	const FULL: &str = "-full";
	const INCREMENTAL: &str = "-incremental";

	#[test]
	fn a_dump_timestamp_becomes_a_comparable_key() {
		assert_eq!(
			key("2026-07-12 00:00:04.001868+00:00"),
			Some(20_260_712_000_004)
		);
	}

	#[test]
	fn a_timestamp_without_sub_second_precision_still_becomes_a_key() {
		assert_eq!(key("2026-07-12 00:00:04"), Some(20_260_712_000_004));
	}

	#[test]
	fn a_timezone_offset_never_reaches_the_key() {
		assert_eq!(
			key("2026-08-21 00:00:03.155180+00:00"),
			key("2026-08-21 00:00:03.999999+05:30")
		);
	}

	#[test]
	fn text_that_is_not_a_timestamp_has_no_key() {
		assert_eq!(key("LATEST"), None);
		assert_eq!(key(""), None);
		assert_eq!(key("2026-07-12"), None);
	}

	#[test]
	fn a_published_name_yields_its_number_and_what_it_reaches() {
		let published = published("listenbrainz-dump-2593-20260712-000004-full", PREFIX, FULL);

		assert_eq!(
			published.as_ref().map(|published| published.number),
			Some(2593)
		);
		assert_eq!(
			published.map(|published| published.reach),
			Some(20_260_712_000_004)
		);
	}

	#[test]
	fn what_a_full_dump_reaches_lines_up_with_the_timestamp_it_carries() {
		assert_eq!(
			published("listenbrainz-dump-2593-20260712-000004-full", PREFIX, FULL)
				.map(|published| published.reach),
			key("2026-07-12 00:00:04.001868+00:00")
		);
	}

	#[test]
	fn what_an_incremental_reaches_lines_up_with_the_timestamp_it_ends_on() {
		assert_eq!(
			published(
				"listenbrainz-dump-2636-20260822-000002-incremental",
				PREFIX,
				INCREMENTAL
			)
			.map(|published| published.reach),
			key("2026-08-22 00:00:02.641933+00:00")
		);
	}

	#[test]
	fn a_name_of_the_other_kind_is_not_published_under_this_suffix() {
		assert!(
			published(
				"listenbrainz-dump-2636-20260822-000002-incremental",
				PREFIX,
				FULL
			)
			.is_none()
		);
		assert!(published("LATEST", PREFIX, FULL).is_none());
	}
}

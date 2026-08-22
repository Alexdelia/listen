use ansi::{
	DIM,
	abbrev::{B, CYA, D, F, G},
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};

use crate::{
	format::{DATE_FORMAT, TIME_FORMAT},
	recommend::island::index::own,
};

use super::{
	age, cache,
	fetch::{Listen, ListenCount},
};

#[derive(Deserialize, Serialize)]
pub(super) struct Held {
	pub dump: String,
	pub covered: i64,
	pub count: ListenCount,
}

pub(super) fn listen(username: &str, refresh: bool) -> hmerr::Result<Option<Held>> {
	let unpacked = own::unpacked()?;

	if let Some(held) = kept(cache::dump::read(username)?, unpacked.as_deref(), refresh) {
		announce(
			&format!("{B}{CYA}cached{D}"),
			username,
			&held,
			&format!(" {DIM}({B}--refresh{D}{DIM} to read the dump again){D}"),
		)?;

		return Ok(Some(held));
	}

	if unpacked.is_none() {
		return Ok(None);
	}

	scanned(username)
}

fn kept(held: Option<Held>, unpacked: Option<&str>, refresh: bool) -> Option<Held> {
	let held = held?;

	match unpacked {
		None => Some(held),
		Some(unpacked) => (!refresh && held.dump == unpacked).then_some(held),
	}
}

fn scanned(username: &str) -> hmerr::Result<Option<Held>> {
	println!("{F}reading own listen off the unpacked dump, once per dump{D}");

	let Some(own) = own::played()? else {
		return Ok(None);
	};

	let held = Held {
		dump: own.dump,
		covered: own.covered,
		count: own
			.play
			.into_iter()
			.map(|play| {
				(
					play.mbid,
					Listen {
						count: play.plays,
						track: play.track,
						artist: play.artist,
					},
				)
			})
			.collect(),
	};

	cache::dump::write(username, &held)?;
	announce(&format!("{B}{G}read{D}"), username, &held, "")?;

	Ok(Some(held))
}

fn announce(source: &str, username: &str, held: &Held, tail: &str) -> hmerr::Result<()> {
	println!(
		"{source} {B}{count}{D} recording off the dump for {B}{username}{D}, \
		covering up to {B}{covered}{D} {DIM}({day} day ago){D}{tail}\n",
		count = held.count.len(),
		covered = covered(held.covered),
		day = age::days_since(held.covered)?
	);

	Ok(())
}

fn covered(covered: i64) -> String {
	DateTime::from_timestamp(covered, 0)
		.map(|at| {
			at.format(&format!("{DATE_FORMAT} {TIME_FORMAT}"))
				.to_string()
		})
		.unwrap_or_default()
}

#[cfg(test)]
mod tests {
	use super::*;

	const DUMP: &str = "2026-07-12 00:00:04.001868+00:00";
	const NEWER: &str = "2026-08-16 00:00:03.000000+00:00";

	fn held() -> Held {
		Held {
			dump: DUMP.to_string(),
			covered: 1_783_814_404,
			count: ListenCount::new(),
		}
	}

	fn dump_of(held: Option<Held>) -> Option<String> {
		held.map(|held| held.dump)
	}

	#[test]
	fn what_was_read_off_the_dump_that_is_still_unpacked_is_read_again_from_the_cache() {
		assert_eq!(
			dump_of(kept(Some(held()), Some(DUMP), false)),
			Some(DUMP.into())
		);
	}

	#[test]
	fn a_newer_unpacked_dump_is_read_rather_than_what_the_cache_holds() {
		assert!(kept(Some(held()), Some(NEWER), false).is_none());
	}

	#[test]
	fn a_refresh_reads_the_unpacked_dump_again() {
		assert!(kept(Some(held()), Some(DUMP), true).is_none());
	}

	#[test]
	fn a_discarded_dump_leaves_the_cache_as_the_only_thing_it_was_read_into() {
		assert_eq!(dump_of(kept(Some(held()), None, false)), Some(DUMP.into()));
		assert_eq!(dump_of(kept(Some(held()), None, true)), Some(DUMP.into()));
	}

	#[test]
	fn nothing_cached_is_nothing_to_keep() {
		assert!(kept(None, Some(DUMP), false).is_none());
		assert!(kept(None, None, false).is_none());
	}

	#[test]
	fn how_far_the_dump_covers_is_told_as_a_date_and_a_time() {
		assert_eq!(covered(1_783_802_344), "2026-07-11 20:39".to_string());
	}
}

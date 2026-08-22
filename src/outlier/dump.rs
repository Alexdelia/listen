use ansi::{
	DIM,
	abbrev::{B, D, F, G},
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};

use crate::{
	format::{DATE_FORMAT, TIME_FORMAT},
	recommend::island::index::own::{self, Gap},
};

use super::{
	age, cache,
	fetch::{Listen, ListenCount},
};

#[derive(Deserialize, Serialize)]
pub(super) struct Held {
	pub dump: String,
	#[serde(default)]
	pub reached: String,
	#[serde(default)]
	pub gap: Vec<Gap>,
	pub covered: i64,
	pub count: ListenCount,
}

impl Held {
	fn reach(&self) -> &str {
		if self.reached.is_empty() {
			return &self.dump;
		}

		&self.reached
	}
}

pub(super) fn listen(username: &str, refresh: bool) -> hmerr::Result<Option<Held>> {
	let Some(mut held) = held(username, refresh)? else {
		return Ok(None);
	};

	folded(username, &mut held)?;
	announce(username, &held)?;

	Ok(Some(held))
}

fn held(username: &str, refresh: bool) -> hmerr::Result<Option<Held>> {
	let unpacked = own::unpacked()?;

	if let Some(held) = kept(cache::dump::read(username)?, unpacked.as_deref(), refresh) {
		return Ok(Some(held));
	}

	if unpacked.is_none() {
		return Ok(None);
	}

	scanned(username)
}

fn folded(username: &str, held: &mut Held) -> hmerr::Result<()> {
	let reached = held.reach().to_string();

	own::fresh(&reached, &mut |fold| {
		absorbed(held, fold);

		cache::dump::write(username, held)
	})
}

fn absorbed(held: &mut Held, fold: own::Fold) {
	merge(&mut held.count, fold.play);
	held.covered = held.covered.max(fold.covered);
	held.gap.extend(fold.gap);
	held.reached = fold.reached;
}

fn merge(count: &mut ListenCount, play: Vec<own::Play>) {
	for play in play {
		let listen = count.entry(play.mbid).or_insert_with(|| Listen {
			count: 0,
			track: play.track,
			artist: play.artist,
		});

		listen.count = listen.count.saturating_add(play.plays);
	}
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
		reached: own.dump.clone(),
		dump: own.dump,
		gap: Vec::new(),
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

	Ok(Some(held))
}

fn announce(username: &str, held: &Held) -> hmerr::Result<()> {
	println!(
		"{B}{G}{count}{D} recording off the dump for {B}{username}{D}, covering up to \
		{B}{covered}{D} {DIM}({day} day ago, {B}--refresh{D}{DIM} to read the dump again){D}\n",
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

	const MBID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

	const DUMP: &str = "2026-07-12 00:00:04.001868+00:00";
	const NEWER: &str = "2026-08-16 00:00:03.000000+00:00";
	const LATEST: &str = "2026-08-22 00:00:02.641933+00:00";

	fn held() -> Held {
		Held {
			dump: DUMP.to_string(),
			reached: String::new(),
			gap: Vec::new(),
			covered: 1_783_814_404,
			count: ListenCount::new(),
		}
	}

	fn play(mbid: &str, plays: u32) -> own::Play {
		own::Play {
			mbid: mbid.parse().unwrap_or_default(),
			plays,
			track: "Fairy Dance".to_string(),
			artist: "UNDEAD CORPORATION".to_string(),
		}
	}

	fn fold(reached: &str, plays: u32, gap: Vec<Gap>) -> own::Fold {
		own::Fold {
			reached: reached.to_string(),
			covered: 0,
			play: vec![play(MBID, plays)],
			gap,
		}
	}

	fn counted(held: &Held, mbid: &str) -> Option<u32> {
		held.count
			.get(&mbid.parse().unwrap_or_default())
			.map(|listen| listen.count)
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
	fn counts_read_before_an_incremental_was_folded_carry_on_from_the_dump_they_came_from() {
		assert_eq!(held().reach(), DUMP);

		let folded = Held {
			reached: NEWER.to_string(),
			..held()
		};

		assert_eq!(folded.reach(), NEWER);
	}

	#[test]
	fn every_incremental_lands_on_the_count_as_it_is_read_not_once_the_chain_is_over() {
		let mut held = held();

		absorbed(&mut held, fold(NEWER, 40, Vec::new()));

		assert_eq!(held.reach(), NEWER);
		assert_eq!(counted(&held, MBID), Some(40));

		absorbed(
			&mut held,
			fold(
				LATEST,
				2,
				vec![Gap {
					from: NEWER.to_string(),
					to: LATEST.to_string(),
				}],
			),
		);

		assert_eq!(held.reach(), LATEST);
		assert_eq!(counted(&held, MBID), Some(42));
		assert_eq!(held.gap.len(), 1);
	}

	#[test]
	fn what_an_incremental_adds_lands_on_the_count_the_dump_left() {
		const FRESH: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

		let mut count = ListenCount::new();
		merge(&mut count, vec![play(MBID, 40)]);
		merge(&mut count, vec![play(MBID, 2), play(FRESH, 7)]);

		assert_eq!(
			count
				.get(&MBID.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(42)
		);
		assert_eq!(
			count
				.get(&FRESH.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(7)
		);
		assert_eq!(
			count
				.get(&FRESH.parse().unwrap_or_default())
				.map(|l| l.track.clone()),
			Some("Fairy Dance".to_string())
		);
	}

	#[test]
	fn how_far_the_dump_covers_is_told_as_a_date_and_a_time() {
		assert_eq!(covered(1_783_802_344), "2026-07-11 20:39".to_string());
	}
}

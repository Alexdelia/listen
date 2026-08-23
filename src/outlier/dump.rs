use ansi::{
	DIM,
	abbrev::{B, D, F, G, Y},
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
	gap,
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
	#[serde(default)]
	pub fold: Option<ListenCount>,
}

struct Carried {
	reached: String,
	gap: Vec<Gap>,
	covered: i64,
	fold: ListenCount,
}

enum Kept {
	Cached(Held),
	Rescan(Option<Carried>),
}

impl Held {
	fn reach(&self) -> &str {
		if self.reached.is_empty() {
			return &self.dump;
		}

		&self.reached
	}

	fn reached_at(&self) -> i64 {
		gap::seconds(self.reach()).unwrap_or(self.covered)
	}

	pub(super) fn ago(&self) -> hmerr::Result<u64> {
		age::days_since(self.reached_at())
	}

	fn foldable(&self) -> bool {
		own::stamped(self.reach())
	}

	pub(super) fn counted(&self) -> ListenCount {
		let mut count = self.count.clone();

		for (mbid, folded) in self.fold.iter().flatten() {
			let listen = count.entry(*mbid).or_insert_with(|| Listen {
				count: 0,
				track: folded.track.clone(),
				artist: folded.artist.clone(),
			});

			listen.count = listen.count.saturating_add(folded.count);
		}

		count
	}

	fn apart(&self) -> bool {
		self.fold.is_some() || self.reach() == self.dump
	}

	fn carried(self) -> Carried {
		Carried {
			reached: self.reach().to_string(),
			gap: self.gap,
			covered: self.covered,
			fold: self.fold.unwrap_or_default(),
		}
	}
}

impl Carried {
	fn of(dump: &str) -> Self {
		Self {
			reached: dump.to_string(),
			gap: Vec::new(),
			covered: 0,
			fold: ListenCount::new(),
		}
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
	let cached = cache::dump::read(username)?;
	let merged = cached.as_ref().is_some_and(|held| !held.apart());
	let stuck = cached
		.as_ref()
		.filter(|held| !held.foldable())
		.map(|held| held.reach().to_string());

	match kept(cached, unpacked.as_deref(), refresh) {
		Kept::Cached(held) => Ok(Some(held)),
		Kept::Rescan(_) if unpacked.is_none() => Ok(None),
		Kept::Rescan(carried) => {
			if carried.is_none() {
				if merged {
					merged_in();
				}

				if let Some(stuck) = stuck {
					stuck_at(&stuck);
				}
			}

			scanned(username, carried)
		}
	}
}

fn merged_in() {
	println!(
		"{Y}the cached count cannot tell the dump from what was folded onto it, \
		reading the dump up again and asking for every incremental since{D}"
	);
}

fn stuck_at(reached: &str) {
	println!(
		"{Y}the counts stopped at {B}{reached}{D}{Y}, which no dump can be held against, \
		reading the dump up again and asking for every incremental since{D}"
	);
}

fn folded(username: &str, held: &mut Held) -> hmerr::Result<()> {
	let reached = held.reach().to_string();

	own::fresh(username, &reached, &mut |fold| {
		absorbed(held, fold);

		cache::dump::write(username, held)
	})
}

fn absorbed(held: &mut Held, fold: own::Fold) {
	merge(held.fold.get_or_insert_default(), fold.play);
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

fn kept(held: Option<Held>, unpacked: Option<&str>, refresh: bool) -> Kept {
	let Some(held) = held else {
		return Kept::Rescan(None);
	};

	let Some(unpacked) = unpacked else {
		return Kept::Cached(held);
	};

	if held.dump != unpacked {
		return Kept::Rescan(None);
	}

	if refresh {
		return Kept::Rescan((held.apart() && held.foldable()).then(|| held.carried()));
	}

	Kept::Cached(held)
}

fn scanned(username: &str, carried: Option<Carried>) -> hmerr::Result<Option<Held>> {
	println!("{F}reading own listen off the unpacked dump, once per dump{D}");

	let Some(own) = own::played(username)? else {
		return Ok(None);
	};

	let carried = carried.unwrap_or_else(|| Carried::of(&own.dump));

	let held = Held {
		dump: own.dump,
		reached: carried.reached,
		gap: carried.gap,
		covered: carried.covered.max(own.covered),
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
		fold: Some(carried.fold),
	};

	cache::dump::write(username, &held)?;

	Ok(Some(held))
}

fn announce(username: &str, held: &Held) -> hmerr::Result<()> {
	println!(
		"{B}{G}{count}{D} recording off the dump for {B}{username}{D}, covering up to \
		{B}{reached}{D} {DIM}({day} day ago, last listen {last}, \
		{B}--refresh{D}{DIM} to read the dump again){D}\n",
		count = held.counted().len(),
		reached = at(held.reached_at()),
		day = held.ago()?,
		last = at(held.covered)
	);

	Ok(())
}

fn at(second: i64) -> String {
	DateTime::from_timestamp(second, 0)
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
			fold: Some(ListenCount::new()),
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

	fn plays(held: &Held, mbid: &str) -> Option<u32> {
		held.counted()
			.get(&mbid.parse().unwrap_or_default())
			.map(|listen| listen.count)
	}

	fn cached_dump(kept: Kept) -> Option<String> {
		match kept {
			Kept::Cached(held) => Some(held.dump),
			Kept::Rescan(_) => None,
		}
	}

	fn carried(kept: Kept) -> Option<Carried> {
		match kept {
			Kept::Cached(_) => None,
			Kept::Rescan(carried) => carried,
		}
	}

	#[test]
	fn what_was_read_off_the_dump_that_is_still_unpacked_is_read_again_from_the_cache() {
		assert_eq!(
			cached_dump(kept(Some(held()), Some(DUMP), false)),
			Some(DUMP.into())
		);
	}

	#[test]
	fn a_newer_unpacked_dump_is_read_rather_than_what_the_cache_holds() {
		assert!(cached_dump(kept(Some(held()), Some(NEWER), false)).is_none());
	}

	#[test]
	fn a_refresh_reads_the_unpacked_dump_again() {
		assert!(cached_dump(kept(Some(held()), Some(DUMP), true)).is_none());
	}

	#[test]
	fn a_refresh_reads_the_dump_again_without_dropping_what_was_folded_onto_it() {
		let mut folded = held();
		absorbed(&mut folded, fold(LATEST, 7, Vec::new()));

		let carried = carried(kept(Some(folded), Some(DUMP), true))
			.unwrap_or_else(|| unreachable!("a refresh of the same dump carries the fold over"));

		assert_eq!(carried.reached, LATEST);
		assert_eq!(
			carried
				.fold
				.get(&MBID.parse().unwrap_or_default())
				.map(|l| l.count),
			Some(7)
		);
	}

	#[test]
	fn an_incremental_that_added_no_play_still_leaves_the_dump_and_the_fold_apart() {
		let mut quiet = held();
		absorbed(
			&mut quiet,
			own::Fold {
				reached: LATEST.to_string(),
				covered: 0,
				play: Vec::new(),
				gap: Vec::new(),
			},
		);

		assert_eq!(quiet.reach(), LATEST);

		let carried = carried(kept(Some(quiet), Some(DUMP), true)).unwrap_or_else(|| {
			unreachable!("an incremental holding nothing of ours is still an incremental read")
		});

		assert_eq!(carried.reached, LATEST);
	}

	#[test]
	fn a_refresh_clears_counts_stopped_at_a_stamp_no_dump_can_be_held_against() {
		let mut wedged = held();
		absorbed(&mut wedged, fold(LATEST, 7, Vec::new()));
		wedged.reached = "END_TIMESTAMP".to_string();

		assert!(!wedged.foldable());
		assert!(carried(kept(Some(wedged), Some(DUMP), true)).is_none());
	}

	#[test]
	fn a_cache_written_before_the_fold_was_kept_apart_is_read_from_the_dump_up_again() {
		let merged = Held {
			reached: LATEST.to_string(),
			fold: None,
			count: ListenCount::from([(
				MBID.parse().unwrap_or_default(),
				Listen {
					count: 47,
					track: String::new(),
					artist: String::new(),
				},
			)]),
			..held()
		};

		assert!(carried(kept(Some(merged), Some(DUMP), true)).is_none());
	}

	#[test]
	fn what_the_dump_counted_and_what_was_folded_onto_it_add_up() {
		let mut held = Held {
			count: ListenCount::from([(
				MBID.parse().unwrap_or_default(),
				Listen {
					count: 30,
					track: "Fairy Dance".to_string(),
					artist: "UNDEAD CORPORATION".to_string(),
				},
			)]),
			..held()
		};

		absorbed(&mut held, fold(LATEST, 5, Vec::new()));

		assert_eq!(plays(&held, MBID), Some(35));
	}

	#[test]
	fn a_newer_dump_is_a_baseline_of_its_own_with_nothing_folded_onto_it_yet() {
		let mut folded = held();
		absorbed(&mut folded, fold(LATEST, 7, Vec::new()));

		assert!(carried(kept(Some(folded), Some(NEWER), false)).is_none());
	}

	#[test]
	fn a_discarded_dump_leaves_the_cache_as_the_only_thing_it_was_read_into() {
		assert_eq!(
			cached_dump(kept(Some(held()), None, false)),
			Some(DUMP.into())
		);
		assert_eq!(
			cached_dump(kept(Some(held()), None, true)),
			Some(DUMP.into())
		);
	}

	#[test]
	fn nothing_cached_is_nothing_to_keep() {
		assert!(cached_dump(kept(None, Some(DUMP), false)).is_none());
		assert!(cached_dump(kept(None, None, false)).is_none());
		assert!(carried(kept(None, Some(DUMP), true)).is_none());
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
		assert_eq!(plays(&held, MBID), Some(40));

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
		assert_eq!(plays(&held, MBID), Some(42));
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
	fn a_timestamp_is_told_as_a_date_and_a_time() {
		assert_eq!(at(1_783_802_344), "2026-07-11 20:39".to_string());
	}

	#[test]
	fn how_far_the_counts_reach_is_the_dump_they_stop_at_not_the_last_listen_they_hold() {
		let quiet = Held {
			reached: LATEST.to_string(),
			..held()
		};

		assert_eq!(quiet.reached_at(), 1_787_356_802);
	}

	#[test]
	fn counts_stopping_at_a_stamp_that_cannot_be_read_fall_back_to_the_last_listen_they_hold() {
		let unreadable = Held {
			reached: "listen".to_string(),
			..held()
		};

		assert_eq!(unreadable.reached_at(), 1_783_814_404);
	}
}

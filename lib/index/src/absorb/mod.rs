mod artist;
mod chain;
mod delta;
#[cfg(test)]
mod fixture;
mod merge;
mod offer;
mod reach;
mod recording;
mod recording_listener;
mod skipped;
mod stage;
mod user_listen;
mod user_stat;
mod work;

use std::path::Path;

use super::{
	board::{Board, Chain},
	decide::Decide,
	dump::{self, Pending},
	index::{self, Meta},
};

use merge::merge;
use offer::{left, resuming};
use reach::taken;
use skipped::skipped;
use work::LIBRARY;

const ABSORBING: &str = "incremental dump to absorb";

pub(super) fn run(
	dir: &Path,
	meta: &Meta,
	pending: &[Pending],
	decide: &dyn Decide,
) -> hmerr::Result<()> {
	let root = dump::root()?;
	let work = work::open(dir, meta.covered())?;
	let mut reach = work::reach(&work, meta);

	let left = left(pending, &reach.covered)?;
	resuming(pending, &left);

	if !dump::offered(&left, ABSORBING, decide)? {
		return Ok(());
	}

	dump::room(&root, &left, chain::AT_ONCE)?;

	let db = index::session::of(&work)?;
	let board = Board::of(
		&stage::PLAN,
		&Chain {
			dump: u64::try_from(left.len()).unwrap_or_default(),
			byte: dump::weight(&left),
		},
	)?;

	chain::each(&board, &root, &left, |incremental| {
		taken(&db, &work, &mut reach, incremental)
	})?;

	if !work::folded(&work, LIBRARY) {
		return skipped(dir, meta, &reach);
	}

	merge(&db, &board, dir, &work, meta, reach)
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{
		fixture::{
			AFTER_A_HOLE, BUILT, DECLARED, FRESH, NEXT, OTHER_RECORDING, OUTSIDER, OWN, POOL_USER,
			POOLED, absorb, artist, built, day, incremental, listen, mbid, one, plays, pooled,
			recording_id, torn,
		},
		*,
	};

	#[test]
	fn a_repeated_recording_carries_the_plays_of_both_the_dump_and_the_incremental() {
		let (dir, index, meta) = built("summed");
		let held = plays(&index, POOLED, 0);

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		assert_eq!(held, 4);
		assert_eq!(plays(&index, POOLED, 0), 6);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_listener_the_index_already_counts_never_counts_twice_for_playing_a_recording_again() {
		let (dir, index, meta) = built("counted");
		let held = pooled(&index, 0, "listener");
		let day = vec![
			listen(POOLED, 0, 2),
			listen(POOLED + 2, FRESH, 1),
			listen(OUTSIDER, 0, 3),
		];

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day));

		assert_eq!(held, i64::from(POOL_USER));
		assert_eq!(
			pooled(&index, 0, "listener"),
			i64::from(POOL_USER),
			"a replay is not a listener, and a listener outside the pool is not one either"
		);
		assert_eq!(
			pooled(&index, 0, "plays"),
			i64::from(POOL_USER) * 4 + 2,
			"the plays of the pool are what grows instead"
		);
		assert_eq!(
			pooled(&index, FRESH, "listener"),
			1,
			"a listener that played it once is a listener"
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_recording_the_index_never_held_takes_an_identifier_after_every_held_one() {
		let (dir, index, meta) = built("fresh");
		let top: i64 = one(&index, "select max(recording_id)::bigint from recording");
		let held = recording_id(&index, 0);

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		assert_eq!(recording_id(&index, FRESH), top + 1);
		assert_eq!(
			recording_id(&index, 0),
			held,
			"a held identifier never moves"
		);
		assert_eq!(
			one::<i64>(&index, "select count(*)::bigint from recording"),
			i64::try_from(DECLARED + OTHER_RECORDING + 1).unwrap_or_default()
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_fresh_recording_carries_the_artists_it_was_credited_to() {
		let (dir, index, meta) = built("credit");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		let credited: i64 = one(
			&index,
			&format!(
				"select count(*)::bigint from read_parquet('{index}/recording_artist.parquet') ra \
				join recording r using (recording_id) where r.mbid = '{mbid}' \
				and ra.artist_mbid = '{artist}'",
				index = index.display(),
				mbid = mbid(FRESH),
				artist = artist(FRESH)
			),
		);

		assert_eq!(credited, 1);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn own_listens_never_reach_the_absorbed_index() {
		let (dir, index, meta) = built("own");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		assert_eq!(
			one::<i64>(
				&index,
				&format!("select count(*)::bigint from user_listen where user_id = {OWN}")
			),
			0
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_pool_the_index_was_built_with_takes_in_nobody_new() {
		let (dir, index, meta) = built("pool");
		let held: i64 = one(&index, "select count(*)::bigint from user_stat");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		assert_eq!(
			one::<i64>(&index, "select count(*)::bigint from user_stat"),
			held
		);
		assert_eq!(
			one::<i64>(
				&index,
				&format!("select count(*)::bigint from user_listen where user_id = {OUTSIDER}")
			),
			0
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_listener_whose_repeats_grew_carries_a_stat_read_off_them() {
		let (dir, index, meta) = built("stat");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));

		let library: i64 = one(
			&index,
			&format!("select recording::bigint from user_stat where user_id = {POOLED}"),
		);
		let listen: i64 = one(
			&index,
			&format!("select count(*)::bigint from user_listen where user_id = {POOLED}"),
		);

		assert_eq!(library, listen, "a stat counts the listens it was read off");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_index_reaches_what_the_dump_it_absorbed_ends_on() {
		let (dir, index, meta) = built("reached");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));
		let meta = index::meta::read(&index).unwrap_or_else(|_| unreachable!());

		assert_eq!(meta.covered(), NEXT);
		assert_eq!(meta.absorbed, 1);
		assert_eq!(
			meta.dump, BUILT,
			"the baseline it was built from never moves"
		);
		assert!(meta.gap.is_empty());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_window_no_dump_covers_is_recorded_as_a_gap() {
		let (dir, index, meta) = built("gap");

		let _ = absorb(&index, &meta, &incremental(&dir, AFTER_A_HOLE, &day()));
		let meta = index::meta::read(&index).unwrap_or_else(|_| unreachable!());

		assert_eq!(meta.gap.len(), 1);
		assert_eq!(meta.gap.first().map(|gap| gap.from.as_str()), Some(BUILT));
		assert_eq!(
			meta.gap.first().map(|gap| gap.to.as_str()),
			Some(AFTER_A_HOLE)
		);
		assert_eq!(meta.covered(), NEXT);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_absorb_that_dies_partway_leaves_the_index_it_would_have_replaced() {
		let (dir, index, meta) = built("survive");
		let held = plays(&index, POOLED, 0);
		let recording: i64 = one(&index, "select count(*)::bigint from recording");

		assert!(absorb(&index, &meta, &torn(&dir)).is_err());

		assert_eq!(plays(&index, POOLED, 0), held);
		assert_eq!(
			one::<i64>(&index, "select count(*)::bigint from recording"),
			recording
		);
		let now = index::meta::read(&index).unwrap_or_else(|_| unreachable!());
		assert_eq!(now.covered(), meta.covered());
		assert_eq!(now.absorbed, 0);
		let _ = fs::remove_dir_all(&dir);
	}
}

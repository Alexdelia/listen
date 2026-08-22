mod artist;
mod board;
mod chain;
mod delta;
mod recording;
mod user_listen;
mod user_stat;
mod work;

use std::path::Path;

use ansi::abbrev::{B, D, F, G, Y};
use chrono::Utc;

use super::{
	board::Board,
	dump::{self, Incremental, Pending},
	open::{self, Gap, Meta},
	progress, query,
};

use work::{LIBRARY, Reach};

pub(super) fn run(dir: &Path, meta: &Meta, pending: &[Pending]) -> hmerr::Result<()> {
	let root = dump::root()?;
	let work = work::open(dir, meta.covered())?;
	let db = open::session(&work)?;
	let mut reach = work::reach(&work, meta);

	let left = left(pending, &reach.covered)?;
	resuming(pending, &left);
	dump::room(&root, &left)?;

	let board = board::of(&board::Chain {
		dump: u64::try_from(left.len()).unwrap_or_default(),
		byte: left.iter().map(|pending| pending.size).sum(),
	})?;

	chain::each(&board, &root, &left, |incremental| {
		taken(&db, &work, &mut reach, incremental)
	})?;

	if !work::folded(&work, LIBRARY) {
		progress::say(format!("{F}nothing absorbed{D}"));

		return Ok(());
	}

	merge(&db, &board, dir, &work, meta, reach)
}

fn left<'a>(pending: &'a [Pending], covered: &str) -> hmerr::Result<Vec<&'a Pending>> {
	let reached = dump::reach(covered)?;

	Ok(pending
		.iter()
		.filter(|pending| pending.reach > reached)
		.collect())
}

fn resuming(pending: &[Pending], left: &[&Pending]) {
	if left.len() == pending.len() {
		return;
	}

	progress::say(format!(
		"{F}{B}{done}{D}{F} of them already absorbed by a previous run, {B}{left}{D}{F} left{D}",
		done = pending.len() - left.len(),
		left = left.len()
	));
}

fn taken(
	db: &duckdb::Connection,
	work: &Path,
	reach: &mut Reach,
	incremental: &Incremental,
) -> hmerr::Result<()> {
	let covered = dump::reach(&reach.covered)?;
	let start = dump::reach(&incremental.start)?;

	if start < covered {
		return overlapping(work, reach, incremental);
	}

	if start > covered {
		lost(reach, &incremental.start);
	}

	delta::fold(db, work, incremental)?;

	reach.covered.clone_from(&incremental.end);
	reach.absorbed += 1;

	work::reached(work, reach)
}

fn lost(reach: &mut Reach, start: &str) {
	progress::say(format!(
		"{Y}nothing published covers {B}{from}{D}{Y} to {B}{to}{D}{Y}, \
		those listens are out of reach{D}",
		from = reach.covered,
		to = start
	));

	reach.gap.push(Gap {
		from: reach.covered.clone(),
		to: start.to_string(),
	});
}

fn overlapping(work: &Path, reach: &mut Reach, incremental: &Incremental) -> hmerr::Result<()> {
	progress::say(format!(
		"{Y}{B}{name}{D}{Y} reaches back into what the index already holds, \
		skipped rather than counted twice{D}",
		name = incremental.name
	));

	reach.gap.push(Gap {
		from: reach.covered.clone(),
		to: incremental.end.clone(),
	});
	reach.covered.clone_from(&incremental.end);

	work::reached(work, reach)
}

fn merge(
	db: &duckdb::Connection,
	board: &Board,
	dir: &Path,
	work: &Path,
	held: &Meta,
	reach: Reach,
) -> hmerr::Result<()> {
	announce(&reach);

	let merge = work::merging(dir, work, &reach.covered)?;

	let recording = recording::of(db, board, &merge)?;
	artist::of(db, board, &merge, &recording)?;
	let row = user_listen::of(db, board, &merge, &recording)?;
	let user = user_stat::of(db, board, &merge)?;

	let meta = Meta {
		built: Utc::now().date_naive().to_string(),
		dump: held.dump.clone(),
		own: held.own,
		reached: Some(reach.covered),
		gap: reach.gap,
		absorbed: held.absorbed + reach.absorbed,
		user,
		recording: query::count(db, &recording)?,
		user_listen: row,
	};

	work::publish(&merge.into, dir, &meta)?;
	work::release(work);

	progress::say(format!(
		"{G}index absorbed up to {B}{to}{D}",
		to = meta.covered()
	));

	Ok(())
}

fn announce(reach: &Reach) {
	progress::say(format!(
		"\n{F}merging {B}{absorbed}{D}{F} absorbed dump into the index{D}\n",
		absorbed = reach.absorbed
	));
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::{
		super::{
			build,
			open::{RECORDING, USER_LISTEN, USER_STAT},
		},
		*,
	};

	const DECLARED: usize = 10;
	const SHARED: usize = 6;
	const OWN: u32 = 1;
	const POOL_USER: u32 = 5;
	const OTHER_RECORDING: usize = 25;
	const FRESH: usize = 100;
	const OUTSIDER: u32 = 99;
	const POOLED: u32 = OWN + 1;

	const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";
	const NEXT: &str = "2026-07-13 00:00:03.000000+00:00";
	const LATER: &str = "2026-07-14 00:00:03.000000+00:00";
	const AFTER_A_HOLE: &str = "2026-07-20 00:00:03.000000+00:00";
	const BEFORE_THE_INDEX: &str = "2026-07-01 00:00:03.000000+00:00";

	fn mbid(recording: usize) -> String {
		format!("00000000-0000-0000-0000-{recording:012x}")
	}

	fn artist(recording: usize) -> String {
		format!("11111111-0000-0000-0000-{recording:012x}")
	}

	fn scratch(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_absorb_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);

		dir
	}

	fn declaration(dir: &Path) -> PathBuf {
		let path = dir.join("listen.ron");
		let entry: Vec<String> = (0..DECLARED)
			.map(|recording| {
				format!(
					"(s: \"{mbid}\", q: {q}, playlist: [])",
					mbid = mbid(recording),
					q = recording % 5
				)
			})
			.collect();
		let _ = fs::write(&path, format!("[{}]", entry.join(",")));

		path
	}

	fn listen(user: u32, recording: usize, plays: usize) -> String {
		(0..plays)
			.map(|_| {
				format!(
					"({user}, '{mbid}', ['{artist}'])",
					mbid = mbid(recording),
					artist = artist(recording)
				)
			})
			.collect::<Vec<_>>()
			.join(",")
	}

	fn shard(into: &Path, row: &[String]) {
		let _ = fs::create_dir_all(into);

		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());
		db.execute_batch(&format!(
			"copy (select * from (values {row}) as t(user_id, recording_mbid, artist_credit_mbids)) \
			to '{into}/0.parquet' (format parquet);",
			row = row.join(","),
			into = into.display()
		))
		.unwrap_or_else(|e| unreachable!("{e}"));
	}

	fn dump(dir: &Path) -> dump::Listen {
		let mut row = Vec::new();
		for recording in 0..DECLARED {
			row.push(listen(OWN, recording, 5));
		}
		for recording in DECLARED..DECLARED + OTHER_RECORDING {
			row.push(listen(OWN, recording, 3));
		}
		for user in 0..POOL_USER {
			let user = POOLED + user;
			for recording in 0..SHARED {
				row.push(listen(user, recording, 4));
			}
			for recording in DECLARED..DECLARED + OTHER_RECORDING {
				row.push(listen(user, recording, 3));
			}
		}

		let into = dir.join("listen");
		shard(&into, &row);

		dump::Listen {
			dir: into,
			name: BUILT.to_string(),
		}
	}

	fn incremental(dir: &Path, start: &str, row: &[String]) -> Incremental {
		let into = dir.join("incremental");
		shard(&into, row);

		Incremental {
			dir: into,
			name: "listenbrainz-dump-2594-20260713-000003-incremental".to_string(),
			start: start.to_string(),
			end: NEXT.to_string(),
		}
	}

	fn following(dir: &Path, row: &[String]) -> Incremental {
		let into = dir.join("following");
		shard(&into, row);

		Incremental {
			dir: into,
			name: "listenbrainz-dump-2595-20260714-000003-incremental".to_string(),
			start: NEXT.to_string(),
			end: LATER.to_string(),
		}
	}

	fn torn(dir: &Path) -> Incremental {
		let into = dir.join("torn");
		let _ = fs::create_dir_all(&into);
		let _ = fs::write(into.join("0.parquet"), b"not a parquet footer");

		Incremental {
			dir: into,
			name: "listenbrainz-dump-2594-20260713-000003-incremental".to_string(),
			start: BUILT.to_string(),
			end: NEXT.to_string(),
		}
	}

	fn day() -> Vec<String> {
		vec![
			listen(POOLED, 0, 2),
			listen(POOLED, FRESH, 3),
			listen(POOLED + 1, FRESH, 3),
			listen(OWN, 0, 4),
			(0..OTHER_RECORDING)
				.map(|recording| listen(OUTSIDER, recording, 3))
				.collect::<Vec<_>>()
				.join(","),
		]
	}

	fn morrow() -> Vec<String> {
		vec![
			listen(POOLED, FRESH + 1, 3),
			listen(POOLED + 1, FRESH + 1, 3),
		]
	}

	fn built(name: &str) -> (PathBuf, PathBuf, Meta) {
		let dir = scratch(name);
		let index = dir.join("index");
		let _ = fs::create_dir_all(&index);

		build::run(&index, &dump(&dir), &declaration(&dir)).unwrap_or_else(|e| unreachable!("{e}"));

		(
			dir.clone(),
			index.clone(),
			open::meta(&index).unwrap_or_else(|_| unreachable!()),
		)
	}

	fn absorb(index: &Path, meta: &Meta, incremental: &Incremental) -> hmerr::Result<Reach> {
		let work = work::open(index, meta.covered())?;
		let db = open::session(&work)?;
		let mut reach = work::reach(&work, meta);
		let board = board::of(&board::Chain { dump: 1, byte: 0 })?;

		taken(&db, &work, &mut reach, incremental)?;

		let held = reach.clone();

		if work::folded(&work, LIBRARY) {
			merge(&db, &board, index, &work, meta, reach)?;
		}

		Ok(held)
	}

	fn one<T: duckdb::types::FromSql>(index: &Path, select: &str) -> T {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());
		db.execute_batch(&format!(
			r"
create view recording as select * from read_parquet('{index}/{RECORDING}');
create view user_listen as select * from read_parquet('{index}/{USER_LISTEN}/*.parquet');
create view user_stat as select * from read_parquet('{index}/{USER_STAT}');
",
			index = index.display()
		))
		.unwrap_or_else(|e| unreachable!("{e}"));

		db.query_row(select, [], |row| row.get(0))
			.unwrap_or_else(|e| unreachable!("{select}\n{e}"))
	}

	fn plays(index: &Path, user: u32, recording: usize) -> i64 {
		one(
			index,
			&format!(
				"select coalesce(sum(ul.plays), 0)::bigint from user_listen ul \
				join recording r using (recording_id) where ul.user_id = {user} \
				and r.mbid = '{mbid}'",
				mbid = mbid(recording)
			),
		)
	}

	fn recording_id(index: &Path, recording: usize) -> i64 {
		one(
			index,
			&format!(
				"select coalesce(max(recording_id), -1)::bigint from recording \
				where mbid = '{mbid}'",
				mbid = mbid(recording)
			),
		)
	}

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
		let meta = open::meta(&index).unwrap_or_else(|_| unreachable!());

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
		let meta = open::meta(&index).unwrap_or_else(|_| unreachable!());

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
	fn a_dump_reaching_back_into_the_index_is_never_counted_twice() {
		let (dir, index, meta) = built("overlap");
		let held = plays(&index, POOLED, 0);

		let reach = absorb(&index, &meta, &incremental(&dir, BEFORE_THE_INDEX, &day()))
			.unwrap_or_else(|_| unreachable!());

		assert_eq!(plays(&index, POOLED, 0), held);
		assert_eq!(reach.absorbed, 0);
		assert_eq!(reach.gap.len(), 1);
		assert_eq!(reach.covered, NEXT);
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
		let now = open::meta(&index).unwrap_or_else(|_| unreachable!());
		assert_eq!(now.covered(), meta.covered());
		assert_eq!(now.absorbed, 0);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_merge_left_half_done_is_redone_over_a_dump_folded_after_it() {
		let (dir, index, meta) = built("resumed");
		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = open::session(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);
		let board =
			board::of(&board::Chain { dump: 2, byte: 0 }).unwrap_or_else(|_| unreachable!());

		taken(&db, &work, &mut reach, &incremental(&dir, BUILT, &day()))
			.unwrap_or_else(|e| unreachable!("{e}"));
		let staged =
			work::merging(&index, &work, &reach.covered).unwrap_or_else(|e| unreachable!("{e}"));
		recording::of(&db, &board, &staged).unwrap_or_else(|e| unreachable!("{e}"));

		taken(&db, &work, &mut reach, &following(&dir, &morrow()))
			.unwrap_or_else(|e| unreachable!("{e}"));
		merge(&db, &board, &index, &work, &meta, reach).unwrap_or_else(|e| unreachable!("{e}"));

		assert_eq!(
			one::<i64>(&index, "select count(*)::bigint from recording"),
			i64::try_from(DECLARED + OTHER_RECORDING + 2).unwrap_or_default(),
			"the dump folded after the merge died reaches the index too"
		);
		assert_eq!(plays(&index, POOLED, FRESH + 1), 3);
		assert_eq!(plays(&index, POOLED, FRESH), 3);
		assert_eq!(
			open::meta(&index)
				.unwrap_or_else(|_| unreachable!())
				.covered(),
			LATER
		);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn absorbing_the_same_dump_again_never_counts_it_twice() {
		let (dir, index, meta) = built("idempotent");

		let _ = absorb(&index, &meta, &incremental(&dir, BUILT, &day()));
		let meta = open::meta(&index).unwrap_or_else(|_| unreachable!());
		let once = plays(&index, POOLED, 0);

		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = open::session(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);
		let again = incremental(&dir, BUILT, &day());

		assert!(taken(&db, &work, &mut reach, &again).is_ok());

		assert_eq!(
			reach.absorbed, 0,
			"a dump the index already reached is skipped"
		);
		assert_eq!(plays(&index, POOLED, 0), once);
		let _ = fs::remove_dir_all(&dir);
	}
}

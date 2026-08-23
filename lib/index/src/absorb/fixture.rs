use std::{
	fs,
	path::{Path, PathBuf},
};

use crate::Seed;

use super::{
	super::{
		build,
		dump::{self, Incremental, Pending},
		open::{self, Meta, RECORDING, RECORDING_LISTENER, USER_LISTEN, USER_STAT},
	},
	board, merge, reach,
	work::{self, LIBRARY, Reach},
};

pub(super) const DECLARED: usize = 10;
pub(super) const SHARED: usize = 6;
pub(super) const OWN: u32 = 1;
pub(super) const POOL_USER: u32 = 5;
pub(super) const OTHER_RECORDING: usize = 25;
pub(super) const FRESH: usize = 100;
pub(super) const OUTSIDER: u32 = 99;
pub(super) const POOLED: u32 = OWN + 1;

pub(super) const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";
pub(super) const NEXT: &str = "2026-07-13 00:00:03.000000+00:00";
pub(super) const LATER: &str = "2026-07-14 00:00:03.000000+00:00";
pub(super) const AFTER_A_HOLE: &str = "2026-07-20 00:00:03.000000+00:00";
pub(super) const BEFORE_THE_INDEX: &str = "2026-07-01 00:00:03.000000+00:00";

pub(super) const FOLDED: u64 = 1 << 20;
pub(super) const WAITING: u64 = 1 << 21;

pub(super) fn mbid(recording: usize) -> String {
	format!("00000000-0000-0000-0000-{recording:012x}")
}

pub(super) fn artist(recording: usize) -> String {
	format!("11111111-0000-0000-0000-{recording:012x}")
}

pub(super) fn scratch(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("declarative_listen_absorb_{name}"));
	let _ = fs::remove_dir_all(&dir);
	let _ = fs::create_dir_all(&dir);

	dir
}

pub(super) fn declaration() -> Vec<Seed> {
	(0..DECLARED)
		.map(|recording| Seed {
			mbid: mbid(recording).parse().unwrap_or_else(|_| unreachable!()),
			q: u8::try_from(recording % 5).unwrap_or_default(),
		})
		.collect()
}

pub(super) fn listen(user: u32, recording: usize, plays: usize) -> String {
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

pub(super) fn shard(into: &Path, row: &[String]) {
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

pub(super) fn dump(dir: &Path) -> dump::Listen {
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

pub(super) fn incremental(dir: &Path, start: &str, row: &[String]) -> Incremental {
	let into = dir.join("incremental");
	shard(&into, row);

	Incremental {
		dir: into,
		name: "listenbrainz-dump-2594-20260713-000003-incremental".to_string(),
		start: start.to_string(),
		end: NEXT.to_string(),
	}
}

pub(super) fn following(dir: &Path, row: &[String]) -> Incremental {
	let into = dir.join("following");
	shard(&into, row);

	Incremental {
		dir: into,
		name: "listenbrainz-dump-2595-20260714-000003-incremental".to_string(),
		start: NEXT.to_string(),
		end: LATER.to_string(),
	}
}

pub(super) fn torn(dir: &Path) -> Incremental {
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

pub(super) fn day() -> Vec<String> {
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

pub(super) fn morrow() -> Vec<String> {
	vec![
		listen(POOLED, FRESH + 1, 3),
		listen(POOLED + 1, FRESH + 1, 3),
	]
}

pub(super) fn waiting(reach: u64, size: u64) -> Pending {
	Pending {
		name: format!("listenbrainz-dump-2594-{reach}-incremental"),
		archive: format!("listenbrainz-spark-dump-{reach}-incremental.tar"),
		size,
		reach,
	}
}

pub(super) fn built(name: &str) -> (PathBuf, PathBuf, Meta) {
	let dir = scratch(name);
	let index = dir.join("index");
	let _ = fs::create_dir_all(&index);

	build::run(&index, &dump(&dir), &declaration()).unwrap_or_else(|e| unreachable!("{e}"));

	(
		dir.clone(),
		index.clone(),
		open::meta(&index).unwrap_or_else(|_| unreachable!()),
	)
}

pub(super) fn absorb(index: &Path, meta: &Meta, incremental: &Incremental) -> hmerr::Result<Reach> {
	let work = work::open(index, meta.covered())?;
	let db = open::session(&work)?;
	let mut reach = work::reach(&work, meta);
	let board = board::of(&board::Chain { dump: 1, byte: 0 })?;

	reach::taken(&db, &work, &mut reach, incremental)?;

	let held = reach.clone();

	if work::folded(&work, LIBRARY) {
		merge::merge(&db, &board, index, &work, meta, reach)?;
	}

	Ok(held)
}

pub(super) fn one<T: duckdb::types::FromSql>(index: &Path, select: &str) -> T {
	let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());
	db.execute_batch(&format!(
		r"
create view recording as select * from read_parquet('{index}/{RECORDING}');
create view recording_listener as select * from read_parquet('{index}/{RECORDING_LISTENER}');
create view user_listen as select * from read_parquet('{index}/{USER_LISTEN}/*.parquet');
create view user_stat as select * from read_parquet('{index}/{USER_STAT}');
",
		index = index.display()
	))
	.unwrap_or_else(|e| unreachable!("{e}"));

	db.query_row(select, [], |row| row.get(0))
		.unwrap_or_else(|e| unreachable!("{select}\n{e}"))
}

pub(super) fn plays(index: &Path, user: u32, recording: usize) -> i64 {
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

pub(super) fn pooled(index: &Path, recording: usize, of: &str) -> i64 {
	one(
		index,
		&format!(
			"select coalesce(max(l.{of}), 0)::bigint from recording_listener l \
			join recording r using (recording_id) where r.mbid = '{mbid}'",
			mbid = mbid(recording)
		),
	)
}

pub(super) fn recording_id(index: &Path, recording: usize) -> i64 {
	one(
		index,
		&format!(
			"select coalesce(max(recording_id), -1)::bigint from recording \
			where mbid = '{mbid}'",
			mbid = mbid(recording)
		),
	)
}

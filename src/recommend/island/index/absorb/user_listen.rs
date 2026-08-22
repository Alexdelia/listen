use std::{fs, path::Path};

use hmerr::ioe;

use super::{
	super::{
		board::Board,
		open::{self, BUCKET, PLAY_CEILING, USER_LISTEN, USER_STAT},
		query,
	},
	board::{self, Stage},
	work::{self, LIBRARY, Merge},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board,
	merge: &Merge,
	recording: &Path,
) -> hmerr::Result<u64> {
	let into = merge.into.join(USER_LISTEN);
	fs::create_dir_all(&into).map_err(|e| ioe!(into.to_string_lossy(), e))?;

	let bar = board::start(board, Stage::Listen)?;

	for bucket in 0..BUCKET {
		let shard = into.join(open::shard(bucket));

		if !query::done(db, &shard) {
			query::copy(db, &shard, &merged(merge, recording, bucket))?;
		}

		bar.inc(1);
	}

	query::count(db, &into.join("*.parquet"))
}

fn merged(merge: &Merge, recording: &Path, bucket: u32) -> String {
	format!(
		r"
with delta as (
	select l.user_id, r.recording_id, sum(l.plays)::ubigint as plays
	from {library} l
	join read_parquet('{recording}') r on r.mbid = l.mbid
	semi join read_parquet('{pool}') u on u.user_id = l.user_id
	where l.user_id % {BUCKET} = {bucket}
	group by 1, 2
),
held as (
	select * from read_parquet('{shard}')
)
select
	coalesce(h.user_id, d.user_id)::uinteger as user_id,
	coalesce(h.recording_id, d.recording_id)::uinteger as recording_id,
	{plays} as plays
from held h
full outer join delta d on d.user_id = h.user_id and d.recording_id = h.recording_id
",
		library = work::read(&merge.work, LIBRARY),
		recording = recording.display(),
		pool = merge.index.join(USER_STAT).display(),
		shard = merge
			.index
			.join(USER_LISTEN)
			.join(open::shard(bucket))
			.display(),
		plays = summed("coalesce(h.plays, 0)", "coalesce(d.plays, 0)")
	)
}

fn summed(held: &str, delta: &str) -> String {
	format!("least({held}::ubigint + {delta}::ubigint, {PLAY_CEILING})::usmallint")
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sum(held: u32, delta: u32) -> u16 {
		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		db.query_row(
			&format!(
				"select {plays}",
				plays = summed(&held.to_string(), &delta.to_string())
			),
			[],
			|row| row.get(0),
		)
		.unwrap_or_else(|e| unreachable!("{e}"))
	}

	#[test]
	fn plays_the_index_holds_and_plays_a_dump_brings_add_up() {
		assert_eq!(sum(4, 2), 6);
		assert_eq!(sum(0, 3), 3);
		assert_eq!(sum(7, 0), 7);
	}

	#[test]
	fn a_count_already_at_the_ceiling_stays_there_instead_of_overflowing() {
		assert_eq!(sum(u32::from(PLAY_CEILING), 9), PLAY_CEILING);
		assert_eq!(sum(u32::from(PLAY_CEILING) - 1, 5), PLAY_CEILING);
	}
}

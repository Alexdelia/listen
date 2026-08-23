use std::fs;

use hmerr::ioe;

use super::{
	super::{
		board::Board,
		index::{
			self,
			layout::{BUCKET, USER_LISTEN, USER_STAT},
		},
		query,
	},
	stage::Stage,
	work::{Merge, STAT},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board<Stage>,
	merge: &Merge,
) -> hmerr::Result<u64> {
	let partial = merge.into.join(STAT);
	fs::create_dir_all(&partial).map_err(|e| ioe!(partial.to_string_lossy(), e))?;

	let bar = board.start(Stage::Stat)?;

	for bucket in 0..BUCKET {
		let shard = partial.join(index::layout::shard(bucket));

		if !query::done(db, &shard) {
			let listen = merge
				.into
				.join(USER_LISTEN)
				.join(index::layout::shard(bucket));
			query::copy(
				db,
				&shard,
				&crate::user_stat::stat(&format!(
					"read_parquet('{listen}')",
					listen = listen.display()
				)),
			)?;
		}

		bar.inc(1);
	}
	drop(bar);

	let into = merge.into.join(USER_STAT);
	let bar = board.start(Stage::UserStat)?;

	if !query::done(db, &into) {
		query::copy(
			db,
			&into,
			&format!(
				"select * from read_parquet('{partial}/*.parquet')",
				partial = partial.display()
			),
		)?;
	}

	bar.inc(1);

	query::count(db, &into)
}

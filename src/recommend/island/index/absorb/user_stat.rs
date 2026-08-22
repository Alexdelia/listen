use std::{fs, path::Path};

use hmerr::ioe;

use super::{
	super::{
		super::attraction,
		board::Board,
		open::{self, BUCKET, USER_LISTEN, USER_STAT},
		query,
	},
	board::{self, Stage},
};

pub(super) fn of(db: &duckdb::Connection, board: &Board, work: &Path) -> hmerr::Result<u64> {
	let partial = work.join(Stage::Stat.title());
	fs::create_dir_all(&partial).map_err(|e| ioe!(partial.to_string_lossy(), e))?;

	let bar = board::start(board, Stage::Stat)?;

	for bucket in 0..BUCKET {
		let shard = partial.join(open::shard(bucket));

		if !query::done(db, &shard) {
			let listen = work.join(USER_LISTEN).join(open::shard(bucket));
			query::copy(
				db,
				&shard,
				&attraction::stat(&format!(
					"read_parquet('{listen}')",
					listen = listen.display()
				)),
			)?;
		}

		bar.inc(1);
	}
	drop(bar);

	let into = work.join(USER_STAT);
	let bar = board::start(board, Stage::UserStat)?;

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

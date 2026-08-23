use std::{fs, path::Path};

use hmerr::ioe;

use super::{
	board::{Board, Planned},
	index::{self, layout::BUCKET},
	query,
};

pub(super) fn step<S: Planned>(
	db: &duckdb::Connection,
	board: &Board<S>,
	stage: S,
	into: &Path,
	select: &str,
) -> hmerr::Result<()> {
	board.run(stage, |bar| {
		if !query::done(db, into) {
			query::copy(db, into, select)?;
		}

		bar.inc(1);

		Ok(())
	})
}

pub(super) fn bucketed<S: Planned>(
	db: &duckdb::Connection,
	board: &Board<S>,
	stage: S,
	into: &Path,
	select: &dyn Fn(u32) -> String,
) -> hmerr::Result<()> {
	fs::create_dir_all(into).map_err(|e| ioe!(into.to_string_lossy(), e))?;

	board.run(stage, |bar| {
		for bucket in 0..BUCKET {
			let shard = into.join(index::layout::shard(bucket));

			if !query::done(db, &shard) {
				query::copy(db, &shard, &select(bucket))?;
			}

			bar.inc(1);
		}

		Ok(())
	})
}

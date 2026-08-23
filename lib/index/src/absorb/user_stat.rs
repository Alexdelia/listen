use super::{
	super::{
		board::Board,
		index::{
			self,
			layout::{USER_LISTEN, USER_STAT},
		},
		part, query,
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

	part::bucketed(db, board, Stage::Stat, &partial, &|bucket| {
		crate::user_stat::stat(&format!(
			"read_parquet('{listen}')",
			listen = merge
				.into
				.join(USER_LISTEN)
				.join(index::layout::shard(bucket))
				.display()
		))
	})?;

	let into = merge.into.join(USER_STAT);

	part::step(
		db,
		board,
		Stage::UserStat,
		&into,
		&format!(
			"select * from read_parquet('{partial}/*.parquet')",
			partial = partial.display()
		),
	)?;

	query::count(db, &into)
}

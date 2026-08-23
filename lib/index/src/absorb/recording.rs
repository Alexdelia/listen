use std::path::PathBuf;

use super::{
	super::{board::Board, open::RECORDING, query},
	stage::Stage,
	work::{self, LIBRARY, Merge},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board<Stage>,
	merge: &Merge,
) -> hmerr::Result<PathBuf> {
	let into = merge.into.join(RECORDING);
	let bar = board.start(Stage::Recording)?;

	if !query::done(db, &into) {
		query::copy(
			db,
			&into,
			&format!(
				r"
with held as (
	select * from read_parquet('{held}')
),
top as (
	select coalesce(max(recording_id), 0) as taken from held
),
delta as (
	select distinct mbid from {library}
),
fresh as (
	select d.mbid from delta d anti join held h on h.mbid = d.mbid
)
select h.recording_id, h.mbid
from held h
union all
select
	((select taken from top) + row_number() over (order by f.mbid))::uinteger as recording_id,
	f.mbid
from fresh f
",
				held = merge.index.join(RECORDING).display(),
				library = work::read(&merge.work, LIBRARY)
			),
		)?;
	}

	bar.inc(1);

	Ok(into)
}

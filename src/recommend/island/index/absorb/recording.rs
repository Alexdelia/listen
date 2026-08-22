use std::path::{Path, PathBuf};

use super::{
	super::{
		board::Board,
		open::{GLOBAL_PLAY_CEILING, RECORDING},
		query,
	},
	board::{self, Stage},
	work::{self, LIBRARY},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board,
	dir: &Path,
	work: &Path,
) -> hmerr::Result<PathBuf> {
	let into = work.join(RECORDING);
	let bar = board::start(board, Stage::Recording)?;

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
	select mbid, sum(plays)::ubigint as plays from {library} group by 1
),
fresh as (
	select d.mbid, d.plays from delta d anti join held h on h.mbid = d.mbid
)
select
	h.recording_id,
	h.mbid,
	least(h.global_plays::ubigint + coalesce(d.plays, 0), {GLOBAL_PLAY_CEILING})::uinteger
		as global_plays
from held h
left join delta d on d.mbid = h.mbid
union all
select
	((select taken from top) + row_number() over (order by f.mbid))::uinteger as recording_id,
	f.mbid,
	least(f.plays, {GLOBAL_PLAY_CEILING})::uinteger as global_plays
from fresh f
",
				held = dir.join(RECORDING).display(),
				library = work::read(work, LIBRARY)
			),
		)?;
	}

	bar.inc(1);

	Ok(into)
}

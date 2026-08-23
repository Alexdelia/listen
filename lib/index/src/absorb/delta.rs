use std::{fs, path::Path};

use hmerr::ioe;

use super::{
	super::{dump::Incremental, play, query, shard},
	work::{self, ARTIST, LIBRARY},
};

pub(super) fn fold(
	db: &duckdb::Connection,
	work: &Path,
	incremental: &Incremental,
) -> hmerr::Result<()> {
	let shard = shard::quoted(&shard::of(&incremental.dir)?.path);

	kept(db, work, LIBRARY, &incremental.name, &play::counted(&shard))?;
	kept(db, work, ARTIST, &incremental.name, &artist(&shard))
}

fn kept(
	db: &duckdb::Connection,
	work: &Path,
	of: &str,
	name: &str,
	select: &str,
) -> hmerr::Result<()> {
	let dir = work::delta(work, of);
	fs::create_dir_all(&dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	let into = dir.join(format!("{name}.parquet"));

	if query::done(db, &into) {
		return Ok(());
	}

	query::copy(db, &into, select)
}

fn artist(shard: &str) -> String {
	format!(
		r"
select distinct
	l.recording_mbid::uuid as mbid,
	unnest(l.artist_credit_mbids)::uuid as artist_mbid
from read_parquet({shard}) l
where l.recording_mbid is not null and l.artist_credit_mbids is not null
"
	)
}

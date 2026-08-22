use std::path::Path;

use super::{
	super::{board::Board, open::RECORDING_ARTIST, query},
	board::{self, Stage},
	work::{self, ARTIST, Merge},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board,
	merge: &Merge,
	recording: &Path,
) -> hmerr::Result<()> {
	let into = merge.into.join(RECORDING_ARTIST);
	let bar = board::start(board, Stage::Credit)?;

	if !query::done(db, &into) {
		query::copy(
			db,
			&into,
			&format!(
				r"
select distinct recording_id, artist_mbid
from (
	select recording_id, artist_mbid from read_parquet('{held}')
	union all
	select r.recording_id, a.artist_mbid
	from {artist} a
	join read_parquet('{recording}') r on r.mbid = a.mbid
)
",
				held = merge.index.join(RECORDING_ARTIST).display(),
				artist = work::read(&merge.work, ARTIST),
				recording = recording.display()
			),
		)?;
	}

	bar.inc(1);

	Ok(())
}

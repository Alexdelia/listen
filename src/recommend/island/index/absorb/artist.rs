use std::path::Path;

use super::{
	super::{board::Board, open::RECORDING_ARTIST, query},
	board::{self, Stage},
	work::{self, ARTIST},
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board,
	dir: &Path,
	work: &Path,
	recording: &Path,
) -> hmerr::Result<()> {
	let into = work.join(RECORDING_ARTIST);
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
				held = dir.join(RECORDING_ARTIST).display(),
				artist = work::read(work, ARTIST),
				recording = recording.display()
			),
		)?;
	}

	bar.inc(1);

	Ok(())
}

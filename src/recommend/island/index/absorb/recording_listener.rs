use super::{
	super::{board::Board, open::RECORDING_LISTENER, query, recording_listener},
	board::{self, Stage},
	work::Merge,
};

pub(super) fn of(db: &duckdb::Connection, board: &Board, merge: &Merge) -> hmerr::Result<()> {
	let into = merge.into.join(RECORDING_LISTENER);
	let bar = board::start(board, Stage::Listener)?;

	if !query::done(db, &into) {
		query::copy(db, &into, &recording_listener::counted(&merge.into))?;
	}

	bar.inc(1);

	Ok(())
}

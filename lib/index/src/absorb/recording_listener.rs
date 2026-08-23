use super::{
	super::{board::Board, index::layout::RECORDING_LISTENER, part, recording_listener},
	stage::Stage,
	work::Merge,
};

pub(super) fn of(
	db: &duckdb::Connection,
	board: &Board<Stage>,
	merge: &Merge,
) -> hmerr::Result<()> {
	part::step(
		db,
		board,
		Stage::Listener,
		&merge.into.join(RECORDING_LISTENER),
		&recording_listener::counted(&merge.into),
	)
}

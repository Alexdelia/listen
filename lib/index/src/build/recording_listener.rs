use std::path::Path;

use super::{
	super::{index::layout::RECORDING_LISTENER, recording_listener},
	scan::Scan,
	stage::Stage,
};

pub(super) fn of(scan: &Scan, dir: &Path) -> miette::Result<()> {
	scan.step(
		Stage::Listener,
		&dir.join(RECORDING_LISTENER),
		&recording_listener::counted(dir),
	)
}

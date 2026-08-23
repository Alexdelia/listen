use std::path::Path;

use super::{
	super::{open::RECORDING_LISTENER, recording_listener},
	scan::Scan,
	stage::Stage,
};

pub(super) fn of(scan: &Scan, dir: &Path) -> hmerr::Result<()> {
	scan.step(
		Stage::Listener,
		&dir.join(RECORDING_LISTENER),
		&recording_listener::counted(dir),
	)
}

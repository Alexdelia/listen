use std::path::Path;

use super::{
	super::{
		board::Board,
		dump::{self, Incremental, Pending},
	},
	stage::Stage,
};

pub(super) fn each(
	board: &Board<Stage>,
	root: &Path,
	pending: &[&Pending],
	fold: impl FnMut(&Incremental) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let downloading = board.start(Stage::Download)?;
	let verifying = board.start(Stage::Verify)?;
	let unpacking = board.start(Stage::Unpack)?;
	let folding = board.start(Stage::Fold)?;

	dump::each(
		root,
		pending,
		&dump::Bar {
			downloading: &downloading,
			verifying: &verifying,
			unpacking: &unpacking,
			folding: &folding,
		},
		fold,
	)
}

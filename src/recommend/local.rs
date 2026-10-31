use std::path::Path;

use listen_index as index;

use miette::miette;

use crate::{
	ask,
	declaration::{Entry, parse},
};

use super::declared;

pub(super) struct Local {
	pub entry: Vec<Entry>,
	pub index: index::Index,
}

pub(super) fn ready() -> bool {
	index::ready()
}

pub(super) fn open(path: &Path) -> miette::Result<Local> {
	let entry = parse::parse(path)?;
	let index = index::ensure(&declared::seed(&entry), &ask::Terminal)?;

	Ok(Local { entry, index })
}

pub(super) fn standing(path: &Path) -> miette::Result<Local> {
	let entry = parse::parse(path)?;
	let index = index::standing()?.ok_or_else(|| {
		miette!(
			help = "run once without --json to build or finish it",
			"no index ready to read as it stands"
		)
	})?;

	Ok(Local { entry, index })
}

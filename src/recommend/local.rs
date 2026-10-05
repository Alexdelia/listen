use std::path::Path;

use listen_index as index;

use ansi::abbrev::{D, R};
use hmerr::ge;

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

pub(super) fn open(path: &Path) -> hmerr::Result<Local> {
	let entry = parse::parse(path)?;
	let index = index::ensure(&declared::seed(&entry), &ask::Terminal)?;

	Ok(Local { entry, index })
}

pub(super) fn standing(path: &Path) -> hmerr::Result<Local> {
	let entry = parse::parse(path)?;
	let index = index::standing()?.ok_or_else(|| {
		ge!(
			format!("{R}no index ready to read as it stands{D}"),
			h: "run once without --json to build or finish it"
		)
	})?;

	Ok(Local { entry, index })
}

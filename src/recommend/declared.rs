use std::{collections::HashSet, path::Path};

use crate::declaration::{Source, parse};

pub(super) fn sources(path: &Path) -> hmerr::Result<HashSet<Source>> {
	Ok(parse::parse(path)?
		.into_iter()
		.map(|entry| entry.s)
		.collect())
}

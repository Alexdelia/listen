use std::collections::HashSet;

use listen_cache::text;

use crate::{cache::root, declaration::Source};

const FILE: &str = "declined";

pub(super) fn load() -> hmerr::Result<HashSet<Source>> {
	let Some(content) = text::read(&root()?.join(FILE))? else {
		return Ok(HashSet::new());
	};

	Ok(content
		.lines()
		.filter_map(|line| line.trim().parse().ok())
		.collect())
}

pub(super) fn add(mbid: Source) -> hmerr::Result<()> {
	text::append(&root()?.join(FILE), &mbid.to_string())
}

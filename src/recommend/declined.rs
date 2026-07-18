use std::{
	collections::HashSet,
	fs::{self, OpenOptions},
	io::Write,
};

use hmerr::ioe;

use crate::{
	cache::{prepare, root},
	declaration::Source,
};

const FILE: &str = "declined";

pub(super) fn load() -> hmerr::Result<HashSet<Source>> {
	let path = root()?.join(FILE);

	if !path.exists() {
		return Ok(HashSet::new());
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(content
		.lines()
		.filter_map(|line| line.trim().parse().ok())
		.collect())
}

pub(super) fn add(mbid: Source) -> hmerr::Result<()> {
	let path = root()?.join(FILE);
	prepare(&path)?;

	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(&path)
		.map_err(|e| ioe!(path.to_string_lossy(), e))?;

	writeln!(file, "{mbid}").map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

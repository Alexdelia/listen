use std::{collections::HashMap, fs, path::PathBuf};

use ansi::abbrev::{B, D, R, Y};
use hmerr::{ge, ioe};

use crate::declaration::Source;

use super::value::Value;

const FILE: &str = "rating";
const EXT: &str = "json";

pub(super) type Submitted = HashMap<Source, Value>;

pub(super) fn read() -> hmerr::Result<Submitted> {
	let path = path()?;

	if !path.exists() {
		return Ok(Submitted::new());
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(serde_json::from_str(&content).unwrap_or_else(|e| {
		eprintln!(
			"{Y}corrupt rating cache {B}{path}{D}{Y}, treating as empty{D}\n{e}",
			path = path.to_string_lossy()
		);
		Submitted::new()
	}))
}

pub(super) fn write(submitted: &Submitted) -> hmerr::Result<()> {
	let path = path()?;
	crate::cache::prepare(&path)?;

	let content = serde_json::to_string(submitted)
		.map_err(|e| ge!(format!("{R}failed to encode cache\n{e}")))?;

	fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn path() -> hmerr::Result<PathBuf> {
	Ok(crate::cache::root()?.join(FILE).with_extension(EXT))
}

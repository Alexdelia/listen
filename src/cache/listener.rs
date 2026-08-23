use std::{fs, path::PathBuf};

use ansi::abbrev::R;
use hmerr::{ge, ioe};
use serde::{Deserialize, Serialize};

use super::{prepare, root};

const SUBDIR: &str = "listener";
const EXT: &str = "json";

#[derive(Deserialize, Serialize)]
pub struct Named {
	pub id: Option<u32>,
	#[serde(default)]
	pub reach: Option<u64>,
}

pub fn read(username: &str) -> hmerr::Result<Option<Named>> {
	let path = path(username)?;

	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(serde_json::from_str(&content).ok())
}

pub fn write(username: &str, named: &Named) -> hmerr::Result<()> {
	let path = path(username)?;
	prepare(&path)?;

	let content =
		serde_json::to_string(named).map_err(|e| ge!(format!("{R}failed to encode cache\n{e}")))?;

	fs::write(&path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

fn path(username: &str) -> hmerr::Result<PathBuf> {
	Ok(root()?.join(SUBDIR).join(username).with_extension(EXT))
}

use std::{fs, path::Path};

use ansi::abbrev::R;
use hmerr::{ge, ioe};
use serde::{Serialize, de::DeserializeOwned};

use crate::prepare;

pub const EXT: &str = "json";

pub fn read<T>(path: &Path) -> hmerr::Result<Option<T>>
where
	T: DeserializeOwned,
{
	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(serde_json::from_str(&content).ok())
}

pub fn write<T>(path: &Path, value: &T) -> hmerr::Result<()>
where
	T: Serialize,
{
	prepare(path)?;

	let content =
		serde_json::to_string(value).map_err(|e| ge!(format!("{R}failed to encode cache\n{e}")))?;

	fs::write(path, content).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(())
}

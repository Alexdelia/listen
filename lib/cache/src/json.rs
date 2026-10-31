use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr};
use serde::{Serialize, de::DeserializeOwned};

use crate::prepare;

pub const EXT: &str = "json";

pub fn read<T>(path: &Path) -> miette::Result<Option<T>>
where
	T: DeserializeOwned,
{
	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	Ok(serde_json::from_str(&content).ok())
}

pub fn write<T>(path: &Path, value: &T) -> miette::Result<()>
where
	T: Serialize,
{
	prepare(path)?;

	let content = serde_json::to_string(value)
		.into_diagnostic()
		.wrap_err("failed to encode cache")?;

	fs::write(path, content)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

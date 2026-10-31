use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr, miette};
use serde::{Deserialize, Serialize};

use super::layout::META;

#[derive(Clone, Deserialize, Serialize)]
pub struct Gap {
	pub from: String,
	pub to: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Meta {
	pub built: String,
	pub dump: String,
	#[serde(default)]
	pub own: Option<u32>,
	#[serde(default)]
	pub reached: Option<String>,
	#[serde(default)]
	pub through: Option<u32>,
	#[serde(default)]
	pub gap: Vec<Gap>,
	#[serde(default)]
	pub absorbed: u32,
	pub user: u64,
	pub recording: u64,
	pub user_listen: u64,
}

impl Meta {
	#[must_use]
	pub fn covered(&self) -> &str {
		self.reached.as_deref().unwrap_or(&self.dump)
	}
}

pub(crate) fn own(dir: &Path) -> Option<u32> {
	read(dir).ok().and_then(|meta| meta.own)
}

pub(crate) fn forget(dir: &Path) -> miette::Result<()> {
	let path = dir.join(META);

	if !path.exists() {
		return Ok(());
	}

	fs::remove_file(&path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot remove {}", path.display()))?;

	Ok(())
}

pub(crate) fn write(dir: &Path, meta: &Meta) -> miette::Result<()> {
	let path = dir.join(META);
	let content = serde_json::to_string(meta).into_diagnostic()?;

	fs::write(&path, content)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

pub(crate) fn read(dir: &Path) -> miette::Result<Meta> {
	let path = dir.join(META);
	let content = fs::read_to_string(&path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	serde_json::from_str(&content).map_err(|e| {
		miette!(
			help = "delete it to rebuild the index",
			"cannot read {}\n{e}",
			path.display()
		)
	})
}

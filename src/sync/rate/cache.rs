use std::{collections::HashMap, fs, path::PathBuf};

use console::style;
use listen_cache::json;
use miette::{IntoDiagnostic, WrapErr};

use crate::declaration::{Source, value::Value};

const FILE: &str = "rating";

pub(super) type Submitted = HashMap<Source, Value>;

pub(super) fn read() -> miette::Result<Submitted> {
	let path = path()?;

	if !path.exists() {
		return Ok(Submitted::new());
	}

	let content = fs::read_to_string(&path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;

	Ok(serde_json::from_str(&content).unwrap_or_else(|e| {
		eprintln!(
			"{}{}{}\n{e}",
			style("corrupt rating cache ").yellow(),
			style(&path.to_string_lossy()).bold().yellow(),
			style(", treating as empty").yellow()
		);
		Submitted::new()
	}))
}

pub(super) fn write(submitted: &Submitted) -> miette::Result<()> {
	json::write(&path()?, submitted)
}

fn path() -> miette::Result<PathBuf> {
	Ok(crate::cache::root()?.join(FILE).with_extension(json::EXT))
}

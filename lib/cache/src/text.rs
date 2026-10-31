use std::{
	fs::{self, OpenOptions},
	io::Write,
	path::Path,
};

use miette::{IntoDiagnostic, WrapErr};

use crate::prepare;

pub fn read(path: &Path) -> miette::Result<Option<String>> {
	if !path.exists() {
		return Ok(None);
	}

	let content = fs::read_to_string(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot read {}", path.display()))?;
	let content = content.trim();

	Ok((!content.is_empty()).then(|| content.to_string()))
}

pub fn write(path: &Path, content: &str) -> miette::Result<()> {
	prepare(path)?;

	fs::write(path, content)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

pub fn append(path: &Path, line: &str) -> miette::Result<()> {
	prepare(path)?;

	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(path)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot open {}", path.display()))?;

	writeln!(file, "{line}")
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

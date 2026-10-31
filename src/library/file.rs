use std::{fs, path::PathBuf};

use miette::{IntoDiagnostic, WrapErr};

pub(super) struct Named {
	pub path: PathBuf,
	pub stem: String,
}

pub(super) fn with_extension(dir: &str, ext: &str) -> miette::Result<Vec<Named>> {
	let mut found = Vec::new();

	for entry in fs::read_dir(dir)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot list {dir}"))?
	{
		let path = entry
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot list {dir}"))?
			.path();

		if !path.is_file() || path.extension().map(|found| found.to_str()) != Some(Some(ext)) {
			continue;
		}

		let Some(stem) = path
			.file_stem()
			.map(|stem| stem.to_string_lossy().to_string())
		else {
			continue;
		};

		found.push(Named { path, stem });
	}

	Ok(found)
}

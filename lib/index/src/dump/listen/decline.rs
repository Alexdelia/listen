use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr};

const DECLINED: &str = "declined-full";

pub(crate) fn declined(root: &Path) -> Option<String> {
	fs::read_to_string(root.join(DECLINED))
		.ok()
		.map(|name| name.trim().to_string())
}

pub(crate) fn decline(root: &Path, dump: &str) -> miette::Result<()> {
	let path = root.join(DECLINED);
	fs::write(&path, dump)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot write {}", path.display()))?;

	Ok(())
}

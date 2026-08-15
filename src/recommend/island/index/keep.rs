use std::{fs, path::Path};

use ansi::abbrev::{B, D, F, Y};
use hmerr::ioe;

use crate::env::{self, Var};

pub(super) fn requested() -> bool {
	env::get_bool(Var::Keep)
}

pub(super) fn discard(path: &Path) -> hmerr::Result<()> {
	if !path.exists() {
		return Ok(());
	}

	if requested() {
		announce(path);
		return Ok(());
	}

	if path.is_dir() {
		fs::remove_dir_all(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	} else {
		fs::remove_file(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	}

	Ok(())
}

fn announce(path: &Path) {
	println!(
		"{F}{key} is set, keeping {B}{Y}{path}{D}",
		key = Var::Keep.key(),
		path = path.display()
	);
}

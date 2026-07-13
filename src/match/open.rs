use std::process::Command;

use ansi::abbrev::{B, D, R};

pub(super) fn open(url: &str) -> hmerr::Result<()> {
	Command::new("xdg-open")
		.arg(url)
		.status()
		.map_err(|e| format!("{R}failed to execute {B}xdg-open{D}\n{e}"))?;

	Ok(())
}

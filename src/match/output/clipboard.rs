use std::process::Command;

use ansi::abbrev::{B, D, R};

pub(super) fn copy(text: &str) -> hmerr::Result<()> {
	Command::new("wl-copy")
		.arg(text)
		.status()
		.map_err(|e| format!("{R}failed to execute {B}wl-copy{D}\n{e}"))?;

	Ok(())
}

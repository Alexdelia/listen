use std::process::Command;

use ansi::abbrev::{B, D, R};
use hmerr::ge;

pub(super) fn copy(text: &str) -> hmerr::Result<()> {
	Command::new("wl-copy")
		.arg(text)
		.status()
		.map_err(|e| ge!(format!("{R}failed to execute {B}wl-copy{D}\n{e}")))?;

	Ok(())
}

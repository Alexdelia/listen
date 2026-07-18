use std::process::Command;

use ansi::abbrev::{B, D, R};
use hmerr::ge;

pub fn open(url: &str) -> hmerr::Result<()> {
	Command::new("xdg-open")
		.arg(url)
		.status()
		.map_err(|e| ge!(format!("{R}failed to execute {B}xdg-open{D}\n{e}")))?;

	Ok(())
}

use std::process::{Command, Stdio};

use ansi::abbrev::{B, D, R};
use hmerr::ge;

pub(crate) fn open(url: &str) -> hmerr::Result<()> {
	let status = Command::new("xdg-open")
		.arg(url)
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_err(|e| ge!(format!("{R}failed to execute {B}xdg-open{D}\n{e}")))?;

	if !status.success() {
		return Err(ge!(format!(
			"{B}xdg-open{D} {R}could not open{D} {B}{url}{D} ({status})"
		))
		.into());
	}

	Ok(())
}

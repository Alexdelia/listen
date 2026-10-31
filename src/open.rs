use std::process::{Command, Stdio};

use miette::{IntoDiagnostic, WrapErr, miette};

pub(crate) fn open(url: &str) -> miette::Result<()> {
	let status = Command::new("xdg-open")
		.arg(url)
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.into_diagnostic()
		.wrap_err("failed to execute xdg-open")?;

	if !status.success() {
		return Err(miette!("xdg-open could not open {url} ({status})"));
	}

	Ok(())
}

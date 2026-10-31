use std::process::Command;

use miette::{IntoDiagnostic, WrapErr};

pub(super) fn copy(text: &str) -> miette::Result<()> {
	Command::new("wl-copy")
		.arg(text)
		.status()
		.into_diagnostic()
		.wrap_err("failed to execute wl-copy")?;

	Ok(())
}

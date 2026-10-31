use miette::{IntoDiagnostic, WrapErr, miette};
use std::process::Command;

pub(super) fn run(command: &mut Command, url: &str) -> miette::Result<()> {
	match command.output() {
		Ok(output) => {
			if output.status.success() {
				return Ok(());
			}

			Err(miette!(
				"failed to download {url}\n{e}",
				e = String::from_utf8_lossy(&output.stderr)
			))
		}
		Err(e) => Err(e).into_diagnostic().wrap_err_with(|| {
			format!(
				"failed to execute {}",
				command.get_program().to_string_lossy()
			)
		}),
	}
}

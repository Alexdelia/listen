use std::{collections::HashMap, path::Path, process::Command};

use miette::{IntoDiagnostic, WrapErr, miette};

use crate::bar;

pub(super) fn scores(
	program: &Path,
	audio: &Path,
	weights: &Path,
) -> miette::Result<HashMap<String, f32>> {
	let spinner = bar::spinner("genre", "cyan")?;
	let output = Command::new(program)
		.arg(audio)
		.arg(weights)
		.output()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {}", program.display()));
	spinner.finish_and_clear();
	let output = output?;

	if !output.status.success() {
		return Err(miette!(
			"failed to score {audio}\n{stderr}",
			audio = audio.display(),
			stderr = String::from_utf8_lossy(&output.stderr).trim()
		));
	}

	serde_json::from_slice(&output.stdout).map_err(|e| {
		miette!(
			"unreadable scores from {program}\n{e}",
			program = program.display()
		)
	})
}

use std::{collections::HashMap, path::Path, process::Command};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::bar;

pub(super) fn scores(
	program: &Path,
	audio: &Path,
	weights: &Path,
) -> hmerr::Result<HashMap<String, f32>> {
	let spinner = bar::spinner("genre", "cyan")?;
	let output = Command::new(program)
		.arg(audio)
		.arg(weights)
		.output()
		.map_err(|e| ioe!(format!("failed to execute {}", program.display()), e));
	spinner.finish_and_clear();
	let output = output?;

	if !output.status.success() {
		return Err(ge!(format!(
			"{R}failed to score {B}{audio}{D}\n{stderr}",
			audio = audio.display(),
			stderr = String::from_utf8_lossy(&output.stderr).trim(),
		))
		.into());
	}

	Ok(serde_json::from_slice(&output.stdout).map_err(|e| {
		ge!(format!(
			"{R}unreadable scores from {B}{program}{D}\n{e}",
			program = program.display()
		))
	})?)
}

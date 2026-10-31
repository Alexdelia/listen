use std::{path::Path, process::Command};

use miette::miette;

use crate::env;

const PROGRAM: &str = "scdl";

pub(super) fn command<P>(url: &str, path: P) -> miette::Result<Command>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();
	let client_id = env::get(env::Var::SoundcloudClientId)?;
	let parent = path
		.parent()
		.ok_or_else(|| miette!("no parent folder for {}", path.display()))?;
	let stem = path
		.file_stem()
		.ok_or_else(|| miette!("no file stem for {}", path.display()))?;

	let mut command = Command::new(PROGRAM);
	command.args([
		"--hide-progress",
		"--client-id",
		&client_id,
		"--onlymp3",
		"--extract-artist",
		"--path",
		parent.to_string_lossy().as_ref(),
		"--name-format",
		stem.to_string_lossy().as_ref(),
		"-l",
		url,
	]);

	Ok(command)
}

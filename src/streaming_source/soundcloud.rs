use std::{path::Path, process::Command};

use crate::env;

use super::StreamingSource;

pub(super) fn command<P>(url: &str, path: P) -> hmerr::Result<Command>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();
	let client_id = env::get(env::Var::SoundcloudClientId)?;
	let parent = path
		.parent()
		.ok_or_else(|| format!("no parent folder for {path}", path = path.to_string_lossy()))?;
	let stem = path
		.file_stem()
		.ok_or_else(|| format!("no file stem for {path}", path = path.to_string_lossy()))?;

	let mut command = Command::new(StreamingSource::SoundCloud.downloader());
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

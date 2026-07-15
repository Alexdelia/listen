use std::{path::Path, process::Command};

use hmerr::ioe;

use crate::env;

use super::StreamingSource;

pub(super) fn download<P>(url: &str, path: P) -> hmerr::Result<()>
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

	match Command::new(StreamingSource::SoundCloud.downloader())
		.args([
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
		])
		.output()
	{
		Ok(output) => {
			if output.status.success() {
				return Ok(());
			}

			Err(format!(
				"failed to download {url}\n{e}",
				e = String::from_utf8_lossy(&output.stderr)
			)
			.into())
		}
		Err(e) => Err(ioe!(
			format!(
				"failed to execute {}",
				StreamingSource::SoundCloud.downloader()
			),
			e,
		)
		.into()),
	}
}

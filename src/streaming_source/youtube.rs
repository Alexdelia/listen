use std::{path::Path, process::Command};

use hmerr::ioe;

use crate::library;

use super::StreamingSource;

pub(super) fn download<P>(url: &str, path: P) -> hmerr::Result<()>
where
	P: AsRef<Path>,
{
	match Command::new(StreamingSource::YouTube.downloader())
		.args([
			"--quiet",
			"--extract-audio",
			"--audio-format",
			library::recording::EXT,
			"--add-metadata",
			"--embed-thumbnail",
			"--ppa",
			"EmbedThumbnail+ffmpeg_o:-c:v png -vf crop=\"'if(gt(ih,iw),iw,ih)':'if(gt(iw,ih),ih,iw)'\"",
			"--output",
			path.as_ref().to_string_lossy().as_ref(),
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
				StreamingSource::YouTube.downloader()
			),
			e,
		)
		.into()),
	}
}

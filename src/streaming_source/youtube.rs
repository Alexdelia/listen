use std::{path::Path, process::Command};

use crate::library;

use super::StreamingSource;

pub(super) fn command<P>(url: &str, path: P) -> Command
where
	P: AsRef<Path>,
{
	let mut command = Command::new(StreamingSource::YouTube.downloader());
	command.args([
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
	]);

	command
}

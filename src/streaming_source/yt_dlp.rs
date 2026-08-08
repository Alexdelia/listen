use std::{path::Path, process::Command};

use crate::library;

const PROGRAM: &str = "yt-dlp";

pub(super) fn command<P>(url: &str, path: P) -> Command
where
	P: AsRef<Path>,
{
	let mut command = Command::new(PROGRAM);
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

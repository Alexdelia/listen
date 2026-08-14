use std::{path::Path, process::Command};

use crate::library;

const PROGRAM: &str = "yt-dlp";

const PLAYER_CLIENT_SERVING_NON_403_URL: &str = "youtube:player_client=web_embedded,default";

pub(super) fn command<P>(url: &str, path: P) -> Command
where
	P: AsRef<Path>,
{
	let mut command = Command::new(PROGRAM);
	command.args([
		"--quiet",
		"--extractor-args",
		PLAYER_CLIENT_SERVING_NON_403_URL,
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

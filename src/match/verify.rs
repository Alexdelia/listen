use std::process::Command;

use ansi::abbrev::{B, D, R};
use hmerr::ge;

use crate::fetch::streaming_source::StreamingSource;

const YT_DLP_ABSENT_FIELD: &str = "NA";

pub(super) fn watch(id: &str) -> String {
	format!(
		"{base_url}/watch?v={id}",
		base_url = StreamingSource::YouTubeMusic.base_url()
	)
}

// a "Song" has track/artist; a "Video" reports NA for both
pub(super) struct Info {
	pub(super) track: Option<String>,
	pub(super) artist: Option<String>,
	pub(super) duration: Option<i64>,
	pub(super) album: Option<String>,
}

impl Info {
	pub(super) fn is_song(&self) -> bool {
		self.track.is_some() && self.artist.is_some()
	}
}

pub(super) fn verify(id: &str) -> hmerr::Result<Option<Info>> {
	let output = Command::new("yt-dlp")
		.args([
			"--skip-download",
			"--no-warnings",
			"--print",
			"%(track,title)s\t%(artist)s\t%(duration)s\t%(album)s",
			&watch(id),
		])
		.output()
		.map_err(|e| ge!(format!("{R}failed to execute {B}yt-dlp{D}\n{e}")))?;

	if !output.status.success() {
		return Ok(None);
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let Some(line) = stdout.lines().next() else {
		return Ok(None);
	};

	let mut field = line.split('\t');

	Ok(Some(Info {
		track: none_if_na(field.next().unwrap_or_default()),
		artist: none_if_na(field.next().unwrap_or_default()),
		duration: field.next().unwrap_or_default().trim().parse().ok(),
		album: none_if_na(field.next().unwrap_or_default()),
	}))
}

fn none_if_na(s: &str) -> Option<String> {
	let s = s.trim();

	(!s.is_empty() && s != YT_DLP_ABSENT_FIELD).then(|| s.to_string())
}

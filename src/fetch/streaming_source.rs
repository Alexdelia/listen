use std::{
	io,
	path::Path,
	process::{Command, Output},
};

use hmerr::ioe;
use musicbrainz_rs_nova::entity::url::Url;

use crate::{entry::Entry, env};

pub enum StreamingSource {
	SoundCloud,
	YouTube,
}

impl StreamingSource {
	pub fn base_url(&self) -> &'static str {
		match self {
			Self::SoundCloud => "https://soundcloud.com",
			Self::YouTube => "https://www.youtube.com",
		}
	}

	fn downloader(&self) -> &'static str {
		match self {
			Self::SoundCloud => "scdl",
			Self::YouTube => "yt-dlp",
		}
	}

	pub fn download<P>(&self, url: &str, path: P) -> hmerr::Result<()>
	where
		P: AsRef<Path>,
	{
		match self {
			Self::SoundCloud => soundcloud(url, path),
			Self::YouTube => youtube(url, path),
		}
	}
}

impl TryFrom<&Url> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &Url) -> Result<Self, Self::Error> {
		if url.resource.starts_with(Self::SoundCloud.base_url()) {
			Ok(Self::SoundCloud)
		} else if url.resource.starts_with(Self::YouTube.base_url()) {
			Ok(Self::YouTube)
		} else {
			Err("unsupported streaming source")
		}
	}
}

fn soundcloud<P>(url: &str, path: P) -> hmerr::Result<()>
where
	P: AsRef<Path>,
{
	match Command::new(StreamingSource::SoundCloud.downloader())
		.args(&[
			"--hide-progress",
			"--client-id",
			&env::get(env::Var::SoundcloudClientId).expect("SOUNDCLOUD_CLIENT_ID not set"),
			"--onlymp3",
			"--extract-artist",
			"--path",
			path.as_ref().to_string_lossy().as_ref(),
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

fn youtube<P>(url: &str, path: P) -> hmerr::Result<()>
where
	P: AsRef<Path>,
{
	match Command::new(StreamingSource::YouTube.downloader())
		.args(&[
			"--quiet",
			"--extract-audio",
			"--audio-format",
			Entry::EXT,
			"--add-metadata",
			"--embed-thumbnail",
			"--ppa",
			"EmbedThumbnail+ffmpeg_o:-c:v png -vf crop=\"'if(gt(ih,iw),iw,ih)':'if(gt(iw,ih),ih,iw)'\"",
			"--output",
			path.as_ref().to_string_lossy().as_ref(),
			url,
		])
		.output() {
			Ok(output) => {
				if output.status.success() {
					return Ok(());
				}

				Err(format!(
					"failed to download {url}\n{e}",
					e = String::from_utf8_lossy(&output.stderr)
				).into())
			},
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

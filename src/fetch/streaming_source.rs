use std::{path::Path, process::Command};

use hmerr::ioe;
use musicbrainz_rs_nova::entity::url::Url;

use crate::{entry::Entry, env};

pub enum StreamingSource {
	SoundCloud,
	YouTube,
	YouTubeMusic,
}

impl StreamingSource {
	pub fn base_url(&self) -> &'static str {
		match self {
			Self::SoundCloud => "https://soundcloud.com",
			Self::YouTube => "https://www.youtube.com",
			Self::YouTubeMusic => "https://music.youtube.com",
		}
	}

	fn downloader(&self) -> &'static str {
		match self {
			Self::SoundCloud => "scdl",
			Self::YouTube => "yt-dlp",
			Self::YouTubeMusic => "yt-dlp",
		}
	}

	pub fn download<P>(&self, url: &str, path: P) -> hmerr::Result<()>
	where
		P: AsRef<Path>,
	{
		match self {
			Self::SoundCloud => soundcloud(url, path),
			Self::YouTube => youtube(url, path),
			Self::YouTubeMusic => youtube(url, path),
		}
	}

	pub fn priority(&self) -> u8 {
		match self {
			Self::SoundCloud => 0,
			Self::YouTubeMusic => 1,
			Self::YouTube => 2,
		}
	}
}

impl TryFrom<&Url> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &Url) -> Result<Self, Self::Error> {
		match &url.resource {
			resource if resource.starts_with(Self::SoundCloud.base_url()) => Ok(Self::SoundCloud),
			resource if resource.starts_with(Self::YouTube.base_url()) => Ok(Self::YouTube),
			resource if resource.starts_with(Self::YouTubeMusic.base_url()) => {
				Ok(Self::YouTubeMusic)
			}
			_ => Err("unsupported streaming source"),
		}
	}
}

fn soundcloud<P>(url: &str, path: P) -> hmerr::Result<()>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();
	match Command::new(StreamingSource::SoundCloud.downloader())
		.args([
			"--hide-progress",
			"--client-id",
			&env::get(env::Var::SoundcloudClientId).expect("SOUNDCLOUD_CLIENT_ID not set"),
			"--onlymp3",
			"--extract-artist",
			"--path",
			path.parent()
				.expect("no parent folder")
				.to_string_lossy()
				.as_ref(),
			"--name-format",
			path.file_stem()
				.expect("no file stem")
				.to_string_lossy()
				.as_ref(),
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
		.args([
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

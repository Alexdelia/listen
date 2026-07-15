use std::{path::Path, process::Command};

use hmerr::ioe;
use musicbrainz_rs::entity::url::Url;

use crate::{env, library};

pub enum StreamingSource {
	SoundCloud,
	YouTube,
	YouTubeMusic,
}

impl StreamingSource {
	pub fn host(&self) -> &'static str {
		match self {
			Self::SoundCloud => "soundcloud.com",
			Self::YouTube => "www.youtube.com",
			Self::YouTubeMusic => "music.youtube.com",
		}
	}

	pub fn base_url(&self) -> String {
		format!("https://{host}", host = self.host())
	}

	fn downloader(&self) -> &'static str {
		match self {
			Self::SoundCloud => "scdl",
			Self::YouTube | Self::YouTubeMusic => "yt-dlp",
		}
	}

	pub fn download<P>(&self, url: &str, path: P) -> hmerr::Result<()>
	where
		P: AsRef<Path>,
	{
		match self {
			Self::SoundCloud => soundcloud(url, path),
			Self::YouTube | Self::YouTubeMusic => youtube(url, path),
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

impl TryFrom<&str> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &str) -> Result<Self, Self::Error> {
		match url {
			url if url.starts_with(Self::SoundCloud.base_url().as_str()) => Ok(Self::SoundCloud),
			url if url.starts_with(Self::YouTube.base_url().as_str()) => Ok(Self::YouTube),
			url if url.starts_with(Self::YouTubeMusic.base_url().as_str()) => {
				Ok(Self::YouTubeMusic)
			}
			_ => Err("unsupported streaming source"),
		}
	}
}

impl TryFrom<&Url> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &Url) -> Result<Self, Self::Error> {
		Self::try_from(url.resource.as_str())
	}
}

fn soundcloud<P>(url: &str, path: P) -> hmerr::Result<()>
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

fn youtube<P>(url: &str, path: P) -> hmerr::Result<()>
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

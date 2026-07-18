mod run;
mod soundcloud;
mod youtube;

use std::path::Path;

use musicbrainz_rs::entity::url::Url;

use run::run;

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
		let mut command = match self {
			Self::SoundCloud => soundcloud::command(url, path)?,
			Self::YouTube | Self::YouTubeMusic => youtube::command(url, path),
		};

		run(&mut command, url)
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

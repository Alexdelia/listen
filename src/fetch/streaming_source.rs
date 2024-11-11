use std::{path::Path, process::Command};

use musicbrainz_rs_nova::entity::url::Url;

use crate::env;

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

	pub fn download<P>(&self, url: &str, path: P)
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

fn soundcloud<P>(url: &str, path: P)
where
	P: AsRef<Path>,
{
	match Command::new(StreamingSource::SoundCloud.downloader())
		.args([
			"--client-id",
			&env::get(env::Var::SoundcloudClientId).expect("SOUNDCLOUD_CLIENT_ID not set"),
			"--onlymp3",
			"--extract-artist",
			"-l",
			url,
		])
		.output()
	{
		Err(e) => {
			dbg!(e);
		}
		Ok(output) => {
			dbg!(output);
		}
	}
}

fn youtube<P>(url: &str, path: P)
where
	P: AsRef<Path>,
{
	// TODO
}

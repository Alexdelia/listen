mod bandcamp;
mod run;
mod soundcloud;
mod yt_dlp;

use std::path::Path;

use musicbrainz_rs::entity::{recording::Recording, relations::RelationContent, url::Url};

use run::run;

pub(crate) use yt_dlp::PROGRAM as YT_DLP;

const STREAMING_RELATION: [&str; 2] = ["free streaming", "streaming"];

pub(crate) fn streaming_url(recording: &Recording) -> impl Iterator<Item = &str> {
	recording
		.relations
		.iter()
		.flatten()
		.filter(|relation| STREAMING_RELATION.contains(&relation.relation_type.as_str()))
		.filter(|relation| !relation.ended.unwrap_or(false))
		.filter_map(|relation| match &relation.content {
			RelationContent::Url(url) => Some(url.resource.as_str()),
			_ => None,
		})
}

pub(crate) enum StreamingSource {
	SoundCloud,
	YouTube,
	YouTubeMusic,
	Bandcamp,
}

const ALL: [StreamingSource; 4] = [
	StreamingSource::SoundCloud,
	StreamingSource::YouTube,
	StreamingSource::YouTubeMusic,
	StreamingSource::Bandcamp,
];

impl StreamingSource {
	pub(crate) const fn host(&self) -> &'static str {
		match self {
			Self::SoundCloud => "soundcloud.com",
			Self::YouTube => "www.youtube.com",
			Self::YouTubeMusic => "music.youtube.com",
			Self::Bandcamp => bandcamp::HOST,
		}
	}

	pub(crate) fn base_url(&self) -> String {
		format!("https://{host}", host = self.host())
	}

	fn serve(&self, url: &str) -> bool {
		match self {
			Self::Bandcamp => bandcamp::is_track(url),
			_ => url.starts_with(self.base_url().as_str()),
		}
	}

	pub(crate) fn download<P>(&self, url: &str, path: P) -> hmerr::Result<()>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		let mut command = match self {
			Self::SoundCloud => soundcloud::command(url, path)?,
			Self::YouTube | Self::YouTubeMusic | Self::Bandcamp => yt_dlp::command(url, path),
		};

		run(&mut command, url)?;

		if !path.exists() {
			return Err(format!(
				"{downloader} exited successfully without downloading {url} to {path}",
				downloader = command.get_program().to_string_lossy(),
				path = path.to_string_lossy(),
			)
			.into());
		}

		Ok(())
	}

	pub(crate) const fn priority(&self) -> u8 {
		match self {
			Self::SoundCloud => 0,
			Self::YouTubeMusic => 1,
			Self::Bandcamp => 2,
			Self::YouTube => 3,
		}
	}
}

impl TryFrom<&str> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &str) -> Result<Self, Self::Error> {
		ALL.into_iter()
			.find(|source| source.serve(url))
			.ok_or("unsupported streaming source")
	}
}

impl TryFrom<&Url> for StreamingSource {
	type Error = &'static str;

	fn try_from(url: &Url) -> Result<Self, Self::Error> {
		Self::try_from(url.resource.as_str())
	}
}

use std::{fs, path::PathBuf};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};
use listen_cache::text;

use crate::{declaration::Source, streaming_source::StreamingSource};

use super::Prefetch;

impl Prefetch {
	pub(crate) fn fetch(&self, source: Source, url: &str) -> hmerr::Result<PathBuf> {
		let complete = self.complete(source);
		let origin = self.origin(source);
		if complete.exists() && text::read(&origin)?.as_deref() == Some(url) {
			return Ok(complete);
		}

		let streaming = StreamingSource::try_from(url)
			.map_err(|e| ge!(format!("{R}cannot prefetch {B}{url}{D}: {e}")))?;

		let partial = self.partial(source);
		listen_cache::prepare(&partial)?;
		if partial.exists() {
			fs::remove_file(&partial).map_err(|e| ioe!(partial.to_string_lossy(), e))?;
		}

		streaming.download(url, &partial)?;

		if origin.exists() {
			fs::remove_file(&origin).map_err(|e| ioe!(origin.to_string_lossy(), e))?;
		}
		fs::rename(&partial, &complete).map_err(|e| ioe!(complete.to_string_lossy(), e))?;
		text::write(&origin, url)?;

		Ok(complete)
	}
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::super::scratch;
	use super::*;

	const URL: &str = "https://unsupported.example/track";

	fn prefetched(prefetch: &Prefetch, source: Source, url: &str) -> PathBuf {
		let complete = prefetch.complete(source);
		listen_cache::prepare(&complete).unwrap();
		fs::write(&complete, b"mp3").unwrap();
		text::write(&prefetch.origin(source), url).unwrap();

		complete
	}

	#[test]
	fn a_recording_already_prefetched_from_the_same_url_is_handed_back_without_downloading() {
		let prefetch = scratch("fetch_reuse");
		let complete = prefetched(&prefetch, Uuid::nil(), URL);

		assert_eq!(prefetch.fetch(Uuid::nil(), URL).unwrap(), complete);
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_recording_prefetched_from_another_url_is_downloaded_again() {
		let prefetch = scratch("fetch_other_url");
		prefetched(&prefetch, Uuid::nil(), "https://unsupported.example/before");

		assert!(prefetch.fetch(Uuid::nil(), URL).is_err());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_recording_prefetched_without_its_url_is_downloaded_again() {
		let prefetch = scratch("fetch_no_origin");
		prefetched(&prefetch, Uuid::nil(), URL);
		fs::remove_file(prefetch.origin(Uuid::nil())).unwrap();

		assert!(prefetch.fetch(Uuid::nil(), URL).is_err());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}
}

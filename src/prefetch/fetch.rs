use std::{fs, path::PathBuf};

use listen_cache::text;
use miette::{IntoDiagnostic, WrapErr, miette};

use crate::{declaration::Source, streaming_source::StreamingSource};

use super::Prefetch;

impl Prefetch {
	pub(crate) fn fetch(&self, source: Source, url: &str) -> miette::Result<PathBuf> {
		let complete = self.complete(source);
		let origin = self.origin(source);
		if complete.exists() && text::read(&origin)?.as_deref() == Some(url) {
			return Ok(complete);
		}

		let streaming =
			StreamingSource::try_from(url).map_err(|e| miette!("cannot prefetch {url}: {e}"))?;

		let partial = self.partial(source);
		listen_cache::prepare(&partial)?;
		if partial.exists() {
			fs::remove_file(&partial)
				.into_diagnostic()
				.wrap_err_with(|| format!("cannot remove {}", partial.display()))?;
		}

		streaming.download(url, &partial)?;

		if origin.exists() {
			fs::remove_file(&origin)
				.into_diagnostic()
				.wrap_err_with(|| format!("cannot remove {}", origin.display()))?;
		}
		fs::rename(&partial, &complete)
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot move into {}", complete.display()))?;
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

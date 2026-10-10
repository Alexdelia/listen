use std::{fs, path::PathBuf};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::{declaration::Source, streaming_source::StreamingSource};

use super::Prefetch;

impl Prefetch {
	pub(crate) fn fetch(&self, source: Source, url: &str) -> hmerr::Result<PathBuf> {
		let complete = self.complete(source);
		if complete.exists() {
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

		fs::rename(&partial, &complete).map_err(|e| ioe!(complete.to_string_lossy(), e))?;

		Ok(complete)
	}
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::super::scratch;

	#[test]
	fn a_recording_already_prefetched_is_handed_back_without_downloading() {
		let prefetch = scratch("fetch_reuse");
		let source = Uuid::nil();
		let complete = prefetch.complete(source);
		listen_cache::prepare(&complete).unwrap();
		std::fs::write(&complete, b"mp3").unwrap();

		assert_eq!(
			prefetch
				.fetch(source, "https://unsupported.example/track")
				.unwrap(),
			complete
		);
		let _ = std::fs::remove_dir_all(&prefetch.dir);
	}
}

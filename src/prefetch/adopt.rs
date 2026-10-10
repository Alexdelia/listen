use std::{fs, io, path::Path};

use hmerr::ioe;

use crate::declaration::Source;

use super::Prefetch;

impl Prefetch {
	pub(crate) fn adopt(&self, source: Source, target: &Path) -> hmerr::Result<bool> {
		let complete = self.complete(source);
		if !complete.is_file() {
			return Ok(false);
		}

		listen_cache::prepare(target)?;
		moved(&complete, target).map_err(|e| ioe!(target.to_string_lossy(), e))?;

		Ok(true)
	}
}

fn moved(from: &Path, to: &Path) -> io::Result<()> {
	match fs::rename(from, to) {
		Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
			fs::copy(from, to)?;
			fs::remove_file(from)
		}
		moved => moved,
	}
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::super::scratch;
	use super::*;

	#[test]
	fn a_prefetched_recording_is_moved_into_a_target_directory_that_does_not_exist_yet() {
		let prefetch = scratch("adopt_complete");
		let source = Uuid::nil();
		let complete = prefetch.complete(source);
		listen_cache::prepare(&complete).unwrap();
		fs::write(&complete, b"mp3").unwrap();
		let target = prefetch
			.dir
			.join("output")
			.join("recording")
			.join("adopted.mp3");

		assert!(prefetch.adopt(source, &target).unwrap());
		assert_eq!(fs::read(&target).unwrap(), b"mp3");
		assert!(!complete.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_recording_still_downloading_is_never_adopted() {
		let prefetch = scratch("adopt_partial");
		let source = Uuid::nil();
		let partial = prefetch.partial(source);
		listen_cache::prepare(&partial).unwrap();
		fs::write(&partial, b"half").unwrap();
		let target = prefetch.dir.join("adopted.mp3");

		assert!(!prefetch.adopt(source, &target).unwrap());
		assert!(!target.exists());
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn nothing_is_adopted_when_nothing_was_ever_prefetched() {
		let prefetch = scratch("adopt_none");

		assert!(
			!prefetch
				.adopt(Uuid::nil(), &prefetch.dir.join("adopted.mp3"))
				.unwrap()
		);
	}
}

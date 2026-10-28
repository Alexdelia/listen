use std::{fs, io, path::Path};

use hmerr::ioe;
use listen_cache::text;

use crate::declaration::Source;

use super::Prefetch;

impl Prefetch {
	pub(crate) fn adopt(
		&self,
		source: Source,
		target: &Path,
		carried: &[&str],
	) -> hmerr::Result<bool> {
		let complete = self.complete(source);
		if !complete.is_file() {
			return Ok(false);
		}

		let origin = self.origin(source);
		let Some(fetched_from) = text::read(&origin)? else {
			return Ok(false);
		};
		if !carried.is_empty() && !carried.contains(&fetched_from.as_str()) {
			return Ok(false);
		}

		listen_cache::prepare(target)?;
		moved(&complete, target).map_err(|e| ioe!(target.to_string_lossy(), e))?;
		fs::remove_file(&origin).map_err(|e| ioe!(origin.to_string_lossy(), e))?;

		Ok(true)
	}
}

const COPYING: &str = "copying";

fn moved(from: &Path, to: &Path) -> io::Result<()> {
	match fs::rename(from, to) {
		Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
			copied(from, to)?;
			fs::remove_file(from)
		}
		moved => moved,
	}
}

fn copied(from: &Path, to: &Path) -> io::Result<()> {
	let copying = to.with_extension(COPYING);

	let landed = fs::copy(from, &copying).and_then(|_| fs::rename(&copying, to));
	if landed.is_err() {
		let _ = fs::remove_file(&copying);
	}

	landed
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::super::scratch;
	use super::*;

	const URL: &str = "https://music.youtube.com/watch?v=settled";
	const OTHER: &str = "https://music.youtube.com/watch?v=other";

	fn prefetched(prefetch: &Prefetch, url: &str) -> std::path::PathBuf {
		let complete = prefetch.complete(Uuid::nil());
		listen_cache::prepare(&complete).unwrap();
		fs::write(&complete, b"mp3").unwrap();
		text::write(&prefetch.origin(Uuid::nil()), url).unwrap();

		complete
	}

	#[test]
	fn a_prefetched_recording_is_moved_into_a_target_directory_that_does_not_exist_yet() {
		let prefetch = scratch("adopt_complete");
		let complete = prefetched(&prefetch, URL);
		let target = prefetch
			.dir
			.join("output")
			.join("recording")
			.join("adopted.mp3");

		assert!(prefetch.adopt(Uuid::nil(), &target, &[OTHER, URL]).unwrap());
		assert_eq!(fs::read(&target).unwrap(), b"mp3");
		assert!(!complete.exists());
		assert!(!prefetch.origin(Uuid::nil()).exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_recording_prefetched_from_a_url_it_no_longer_carries_is_left_alone() {
		let prefetch = scratch("adopt_stale_url");
		let complete = prefetched(&prefetch, URL);
		let target = prefetch.dir.join("adopted.mp3");

		assert!(!prefetch.adopt(Uuid::nil(), &target, &[OTHER]).unwrap());
		assert!(!target.exists());
		assert!(complete.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_recording_carrying_no_url_yet_adopts_what_the_match_prefetched() {
		let prefetch = scratch("adopt_unlinked");
		prefetched(&prefetch, URL);
		let target = prefetch.dir.join("adopted.mp3");

		assert!(prefetch.adopt(Uuid::nil(), &target, &[]).unwrap());
		assert_eq!(fs::read(&target).unwrap(), b"mp3");
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_prefetched_recording_whose_url_was_never_recorded_is_left_alone() {
		let prefetch = scratch("adopt_no_origin");
		prefetched(&prefetch, URL);
		fs::remove_file(prefetch.origin(Uuid::nil())).unwrap();
		let target = prefetch.dir.join("adopted.mp3");

		assert!(!prefetch.adopt(Uuid::nil(), &target, &[]).unwrap());
		assert!(!target.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_copy_lands_under_its_target_name_only_once_complete() {
		let prefetch = scratch("adopt_copy");
		let from = prefetch.dir.join("from.mp3");
		let to = prefetch.dir.join("to.mp3");
		listen_cache::prepare(&from).unwrap();
		fs::write(&from, b"mp3").unwrap();

		copied(&from, &to).unwrap();

		assert_eq!(fs::read(&to).unwrap(), b"mp3");
		assert!(!to.with_extension(COPYING).exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn a_copy_that_fails_leaves_nothing_under_the_target_name() {
		let prefetch = scratch("adopt_copy_failed");
		let to = prefetch.dir.join("to.mp3");
		listen_cache::prepare(&to).unwrap();

		assert!(copied(&prefetch.dir.join("missing.mp3"), &to).is_err());
		assert!(!to.exists());
		assert!(!to.with_extension(COPYING).exists());
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

		assert!(!prefetch.adopt(source, &target, &[]).unwrap());
		assert!(!target.exists());
		assert!(partial.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn nothing_is_adopted_when_nothing_was_ever_prefetched() {
		let prefetch = scratch("adopt_none");

		assert!(
			!prefetch
				.adopt(Uuid::nil(), &prefetch.dir.join("adopted.mp3"), &[])
				.unwrap()
		);
	}
}

use std::path::Path;

use super::layout::{
	ARTIST_LINK, BUCKET, META, RECORDING, RECORDING_ARTIST, RECORDING_LISTENER, USER_LISTEN,
	USER_STAT, shard,
};

fn indexed(dir: &Path) -> bool {
	scanned(dir) && dir.join(USER_STAT).exists()
}

pub(crate) fn predates_stat(dir: &Path) -> bool {
	scanned(dir) && !dir.join(USER_STAT).exists()
}

pub(crate) fn predates_listener(dir: &Path) -> bool {
	scanned(dir) && !dir.join(RECORDING_LISTENER).exists()
}

pub(crate) fn scanned(dir: &Path) -> bool {
	[RECORDING, RECORDING_ARTIST, META]
		.iter()
		.all(|part| dir.join(part).exists())
		&& bucketed(&dir.join(USER_LISTEN))
}

fn bucketed(into: &Path) -> bool {
	(0..BUCKET).all(|bucket| into.join(shard(bucket)).exists())
}

pub(crate) fn built(dir: &Path) -> bool {
	indexed(dir) && dir.join(ARTIST_LINK).exists()
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{super::meta::forget, *};

	fn lay_out(dir: &Path, bucket: u32) {
		let into = dir.join(USER_LISTEN);
		let _ = fs::create_dir_all(&into);

		for part in [
			RECORDING,
			RECORDING_ARTIST,
			RECORDING_LISTENER,
			USER_STAT,
			META,
		] {
			let _ = fs::write(dir.join(part), b"built");
		}
		for bucket in 0..bucket {
			let _ = fs::write(into.join(shard(bucket)), b"built");
		}
	}

	#[test]
	fn every_bucket_alongside_the_written_meta_is_an_index() {
		let dir = crate::scratch::of("index", "whole");
		lay_out(&dir, BUCKET);

		assert!(indexed(&dir));
		assert!(!predates_stat(&dir));
		assert!(!predates_listener(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_written_before_the_listener_stat_is_one_to_recover() {
		let dir = crate::scratch::of("index", "predate");
		lay_out(&dir, BUCKET);
		let _ = fs::remove_file(dir.join(USER_STAT));

		assert!(!indexed(&dir));
		assert!(predates_stat(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_written_before_the_listener_count_still_counts_as_one() {
		let dir = crate::scratch::of("index", "uncounted");
		lay_out(&dir, BUCKET);
		let _ = fs::remove_file(dir.join(RECORDING_LISTENER));

		assert!(indexed(&dir));
		assert!(predates_listener(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_stat_cannot_be_recovered_from_listens_that_are_not_all_there() {
		let dir = crate::scratch::of("index", "half");
		lay_out(&dir, BUCKET / 2);
		let _ = fs::remove_file(dir.join(USER_STAT));

		assert!(!predates_stat(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_build_interrupted_partway_through_the_buckets_is_not_an_index() {
		let dir = crate::scratch::of("index", "interrupted");
		lay_out(&dir, BUCKET / 2);

		assert!(!indexed(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_whose_meta_is_forgotten_is_built_again() {
		let dir = crate::scratch::of("index", "forgotten");
		lay_out(&dir, BUCKET);

		assert!(forget(&dir).is_ok());

		assert!(!dir.join(META).exists());
		assert!(!indexed(&dir));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn forgetting_a_meta_that_was_never_written_is_not_an_error() {
		let dir = crate::scratch::of("index", "never");

		assert!(forget(&dir).is_ok());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_without_its_artist_link_is_not_built_yet() {
		let dir = crate::scratch::of("index", "link");
		lay_out(&dir, BUCKET);

		assert!(!built(&dir));

		let _ = fs::write(dir.join(ARTIST_LINK), b"built");

		assert!(built(&dir));
		let _ = fs::remove_dir_all(&dir);
	}
}

use std::{
	fs, io,
	path::Path,
	time::{Duration, SystemTime},
};

use hmerr::ioe;

use super::{PARTIAL, Prefetch};

const PRUNE_AFTER: Duration = Duration::from_hours(30 * 24);

impl Prefetch {
	pub(crate) fn prune(&self) -> hmerr::Result<()> {
		let Some(cutoff) = SystemTime::now().checked_sub(PRUNE_AFTER) else {
			return Ok(());
		};

		prune_in(&self.dir, cutoff)?;
		prune_in(&self.dir.join(PARTIAL), cutoff)
	}
}

fn prune_in(dir: &Path, cutoff: SystemTime) -> hmerr::Result<()> {
	if !dir.is_dir() {
		return Ok(());
	}

	for entry in fs::read_dir(dir).map_err(|e| ioe!(dir.to_string_lossy(), e))? {
		let path = entry.map_err(|e| ioe!(dir.to_string_lossy(), e))?.path();
		let metadata = match fs::metadata(&path) {
			Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
			metadata => metadata.map_err(|e| ioe!(path.to_string_lossy(), e))?,
		};

		if !metadata.is_file() {
			continue;
		}

		let modified = metadata
			.modified()
			.map_err(|e| ioe!(path.to_string_lossy(), e))?;
		if modified < cutoff {
			remove_if_present(&path)?;
		}
	}

	Ok(())
}

fn remove_if_present(path: &Path) -> hmerr::Result<()> {
	match fs::remove_file(path) {
		Err(e) if e.kind() != io::ErrorKind::NotFound => Err(ioe!(path.to_string_lossy(), e))?,
		_ => Ok(()),
	}
}

#[cfg(test)]
mod tests {
	use std::fs::File;

	use uuid::Uuid;

	use super::super::scratch;
	use super::*;

	fn aged(path: &Path, age: Duration) {
		listen_cache::prepare(path).unwrap();
		fs::write(path, b"mp3").unwrap();
		File::options()
			.write(true)
			.open(path)
			.unwrap()
			.set_modified(SystemTime::now() - age)
			.unwrap();
	}

	#[test]
	fn only_entries_older_than_thirty_days_are_pruned_partial_ones_included() {
		let prefetch = scratch("prune");
		let day = Duration::from_hours(24);
		let stale = prefetch.complete(Uuid::nil());
		let stale_partial = prefetch.partial(Uuid::nil());
		let fresh = prefetch.complete(Uuid::max());
		aged(&stale, 31 * day);
		aged(&stale_partial, 31 * day);
		aged(&fresh, 29 * day);

		prefetch.prune().unwrap();

		assert!(!stale.exists());
		assert!(!stale_partial.exists());
		assert!(fresh.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn an_entry_gone_before_it_could_be_read_does_not_stop_the_rest_from_being_pruned() {
		let prefetch = scratch("prune_vanished");
		let stale = prefetch.complete(Uuid::nil());
		aged(&stale, Duration::from_hours(31 * 24));
		std::os::unix::fs::symlink(
			prefetch.dir.join("adopted_meanwhile"),
			prefetch.complete(Uuid::max()),
		)
		.unwrap();

		prefetch.prune().unwrap();

		assert!(!stale.exists());
		let _ = fs::remove_dir_all(&prefetch.dir);
	}

	#[test]
	fn removing_what_is_already_gone_is_no_error() {
		let prefetch = scratch("prune_remove_gone");

		assert!(remove_if_present(&prefetch.complete(Uuid::nil())).is_ok());
	}

	#[test]
	fn pruning_a_prefetch_directory_that_does_not_exist_does_nothing() {
		assert!(scratch("prune_none").prune().is_ok());
	}
}

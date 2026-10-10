use std::{
	fs,
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
		let metadata = fs::metadata(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

		if !metadata.is_file() {
			continue;
		}

		let modified = metadata
			.modified()
			.map_err(|e| ioe!(path.to_string_lossy(), e))?;
		if modified < cutoff {
			fs::remove_file(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
		}
	}

	Ok(())
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
	fn pruning_a_prefetch_directory_that_does_not_exist_does_nothing() {
		assert!(scratch("prune_none").prune().is_ok());
	}
}

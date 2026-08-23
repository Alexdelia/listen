use std::{fs, path::Path};

use ansi::abbrev::{B, D, F, Y};

use super::{
	super::super::{keep, progress},
	Listen,
};

pub(crate) fn discard(listen: &Listen) -> hmerr::Result<()> {
	if !listen.dir.is_dir() {
		return Ok(());
	}

	if !keep::requested() {
		progress::say(format!(
			"{F}index built, releasing its {B}{Y}{size}{D}{F} dump{D}",
			size = progress::bytes(weight(&listen.dir))
		));
	}

	keep::discard(&listen.dir)
}

fn weight(dir: &Path) -> u64 {
	let Ok(read) = fs::read_dir(dir) else {
		return 0;
	};

	read.filter_map(Result::ok)
		.filter_map(|entry| entry.metadata().ok())
		.filter(std::fs::Metadata::is_file)
		.map(|meta| meta.len())
		.sum()
}

#[cfg(test)]
mod tests {
	use super::{
		super::{
			LISTEN,
			fixture::{listen, scratch},
		},
		*,
	};

	#[test]
	fn discarding_removes_the_dump() {
		let root = scratch("discard");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"payload");

		let _ = discard(&listen(dir.clone()));

		assert!(!dir.exists());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn discarding_a_linked_dump_leaves_what_it_points_at_alone() {
		let root = scratch("linked");
		let dir = root.join(LISTEN);
		let elsewhere = root.join("elsewhere.parquet");
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(&elsewhere, b"payload");
		let _ = std::os::unix::fs::symlink(&elsewhere, dir.join("0.parquet"));

		let _ = discard(&listen(dir.clone()));

		assert!(!dir.exists());
		assert!(elsewhere.exists(), "the link target must survive");
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn discarding_what_is_already_gone_is_not_an_error() {
		let root = scratch("gone");

		assert!(discard(&listen(root.join(LISTEN))).is_ok());
		let _ = fs::remove_dir_all(&root);
	}
}

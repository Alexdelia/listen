use std::{fs, path::Path};

use hmerr::ioe;

use super::{LISTEN, Listen};

const STAMP: &str = "TIMESTAMP";

pub(crate) fn find(root: &Path) -> hmerr::Result<Option<Listen>> {
	let dir = root.join(LISTEN);

	if !holds_parquet(&dir)? {
		return Ok(None);
	}

	Ok(Some(Listen {
		name: name_of(&dir),
		dir,
	}))
}

pub(super) fn name_of(dir: &Path) -> String {
	timestamp(dir).unwrap_or_else(|| {
		dir.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default()
	})
}

pub(super) fn timestamp(dir: &Path) -> Option<String> {
	fs::read_to_string(dir.join(STAMP))
		.ok()
		.map(|stamp| stamp.trim().to_string())
}

fn holds_parquet(dir: &Path) -> hmerr::Result<bool> {
	if !dir.is_dir() {
		return Ok(false);
	}

	for entry in fs::read_dir(dir).map_err(|e| ioe!(dir.to_string_lossy(), e))? {
		let entry = entry.map_err(|e| ioe!(dir.to_string_lossy(), e))?;
		if entry.path().extension().is_some_and(|ext| ext == "parquet") {
			return Ok(true);
		}
	}

	Ok(false)
}

#[cfg(test)]
mod tests {
	use super::{super::fixture::scratch, *};

	#[test]
	fn a_directory_without_parquet_is_not_a_dump() {
		let root = scratch("empty");
		let _ = fs::create_dir_all(root.join(LISTEN));

		assert!(find(&root).unwrap_or_default().is_none());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn a_directory_holding_parquet_is_a_dump() {
		let root = scratch("holding");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"");

		assert!(find(&root).unwrap_or_default().is_some());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn the_dump_is_named_after_its_timestamp() {
		let root = scratch("stamp");
		let dir = root.join(LISTEN);
		let _ = fs::create_dir_all(&dir);
		let _ = fs::write(dir.join("0.parquet"), b"");
		let _ = fs::write(dir.join(STAMP), b"  20260712-000004\n");

		assert_eq!(
			find(&root).unwrap_or_default().map(|dump| dump.name),
			Some("20260712-000004".to_string())
		);
		let _ = fs::remove_dir_all(&root);
	}
}

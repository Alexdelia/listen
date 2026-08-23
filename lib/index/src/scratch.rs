use std::{fs, path::PathBuf};

pub(crate) fn of(prefix: &str, name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("declarative_listen_{prefix}_{name}"));
	let _ = fs::remove_dir_all(&dir);
	let _ = fs::create_dir_all(&dir);

	dir
}

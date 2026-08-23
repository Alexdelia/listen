use std::{fs, path::PathBuf};

use super::Listen;

pub(super) fn scratch(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("declarative_listen_dump_{name}"));
	let _ = fs::remove_dir_all(&dir);
	let _ = fs::create_dir_all(&dir);

	dir
}

pub(super) fn listen(dir: PathBuf) -> Listen {
	Listen {
		name: "test".to_string(),
		dir,
	}
}

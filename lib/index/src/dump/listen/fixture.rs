use std::path::PathBuf;

use super::Listen;

pub(super) fn scratch(name: &str) -> PathBuf {
	crate::scratch::of("dump", name)
}

pub(super) fn listen(dir: PathBuf) -> Listen {
	Listen {
		name: "test".to_string(),
		dir,
	}
}

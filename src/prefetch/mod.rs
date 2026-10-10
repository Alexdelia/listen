mod adopt;
mod fetch;
mod prune;

use std::path::{Path, PathBuf};

use crate::{declaration::Source, library};

const DIR: &str = "prefetch";
const PARTIAL: &str = "partial";

pub(crate) struct Prefetch {
	dir: PathBuf,
}

impl Prefetch {
	pub(crate) fn cached() -> hmerr::Result<Self> {
		Ok(Self::at(listen_cache::root()?.join(DIR)))
	}

	const fn at(dir: PathBuf) -> Self {
		Self { dir }
	}

	fn complete(&self, source: Source) -> PathBuf {
		recording_in(&self.dir, source)
	}

	fn partial(&self, source: Source) -> PathBuf {
		recording_in(&self.dir.join(PARTIAL), source)
	}
}

fn recording_in(dir: &Path, source: Source) -> PathBuf {
	dir.join(source.to_string())
		.with_extension(library::recording::EXT)
}

#[cfg(test)]
fn scratch(name: &str) -> Prefetch {
	let dir = std::env::temp_dir().join(format!("declarative_listen_prefetch_{name}"));
	let _ = std::fs::remove_dir_all(&dir);

	Prefetch::at(dir)
}

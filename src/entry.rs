use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub type Source = String;
pub type Q = u8;

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
	pub s: Source,
	pub q: Q,
	pub playlist: Vec<String>,
}

impl Entry {
	pub const OUTPUT_DIR: &'static str = "./output/recording";

	pub const EXT: &'static str = "mp3";

	pub fn path_from_source(source: &str) -> PathBuf {
		Path::new(Self::OUTPUT_DIR)
			.join(source)
			.with_extension(Self::EXT)
	}
}

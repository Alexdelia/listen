use std::path::Path;

use super::{output, verify::Info};

pub(super) fn run(path: &Path, mbid: &str, info: Option<&Info>, length: i64) -> hmerr::Result<()> {
	if let Some(info) = info {
		output::found(info, length);
	}

	output::entry(path, mbid)
}

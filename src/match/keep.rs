use std::path::Path;

use super::{output, verify::Info};

pub(super) fn run(path: &Path, mbid: &str, info: &Info, length: i64) -> hmerr::Result<()> {
	output::found(info, length);
	output::entry(path, mbid)
}

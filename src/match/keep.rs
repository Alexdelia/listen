use std::path::Path;

use super::{output, verify::Info};

pub(super) fn run(
	path: &Path,
	mbid: &str,
	found: Option<(&Info, &str)>,
	length: i64,
) -> hmerr::Result<()> {
	if let Some((info, url)) = found {
		output::found(info, length);
		output::url(url);
	}

	output::entry(path, mbid)
}

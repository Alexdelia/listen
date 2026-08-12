use std::path::Path;

use super::{declare, open, output, verify::Info};

pub(super) fn run(
	path: &Path,
	mbid: &str,
	found: Option<(&Info, &str)>,
	length: i64,
	recommend: bool,
) -> hmerr::Result<bool> {
	if let Some((info, url)) = found {
		output::found(info, length);
		output::url(url);

		if recommend {
			open::open(url)?;
		}
	}

	declare::run(path, mbid, recommend)
}

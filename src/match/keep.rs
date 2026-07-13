use super::{output, verify::Info};

pub(super) fn run(mbid: &str, info: &Info, length: i64) -> hmerr::Result<()> {
	output::found(info, length);
	output::entry(mbid);

	Ok(())
}

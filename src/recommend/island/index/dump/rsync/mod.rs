mod digest;
mod fetch;
mod list;

use std::{path::Path, process::Command};

use ansi::abbrev::{B, D, R};
use hmerr::{GenericError, ge};

pub(super) use digest::checksum;
pub(super) use fetch::{latest_marker, pull};
pub(super) use list::{Entry, beneath, biggest, list};

const PROGRAM: &str = "rsync";
pub(super) const HOST: &str = "rsync://data.metabrainz.org/musicbrainz";

pub(super) fn checked(url: &str, root: &Path, checksum: &str) -> hmerr::Result<()> {
	fetch::small(&format!("{url}{checksum}"), &root.join(checksum))?;
	digest::verify(root, checksum)?;

	fetch::forget(root, &[checksum])
}

fn ran(argument: &[&str], attempt: &str, url: &str) -> hmerr::Result<Vec<u8>> {
	let out = Command::new(PROGRAM)
		.args(argument)
		.output()
		.map_err(|e| missing_rsync(&e.to_string()))?;

	if !out.status.success() {
		return Err(ge!(format!(
			"{R}{B}{PROGRAM}{D}{R} could not {attempt} {B}{url}{D}\n{}{D}",
			String::from_utf8_lossy(&out.stderr).trim()
		))
		.into());
	}

	Ok(out.stdout)
}

fn missing_rsync(reason: &str) -> GenericError {
	ge!(
		format!("{R}failed to execute {B}{PROGRAM}{D}\n{reason}"),
		h: format!("{B}{PROGRAM}{D} downloads the dump, it comes with the nix dev shell")
	)
}

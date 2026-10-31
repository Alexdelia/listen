mod digest;
mod fetch;
mod list;

use std::{path::Path, process::Command};

use miette::miette;

pub(super) use digest::checksum;
pub(super) use fetch::{latest_marker, pull};
pub(super) use list::{Entry, beneath, biggest, list};

const PROGRAM: &str = "rsync";
pub(super) const HOST: &str = "rsync://data.metabrainz.org/musicbrainz";

pub(super) fn checked(url: &str, root: &Path, checksum: &str) -> miette::Result<()> {
	fetch::small(&format!("{url}{checksum}"), &root.join(checksum))?;
	digest::verify(root, checksum)?;

	fetch::forget(root, &[checksum])
}

fn ran(argument: &[&str], attempt: &str, url: &str) -> miette::Result<Vec<u8>> {
	let out = Command::new(PROGRAM)
		.args(argument)
		.output()
		.map_err(|e| missing_rsync(&e.to_string()))?;

	if !out.status.success() {
		return Err(miette!(
			"{PROGRAM} could not {attempt} {url}\n{}",
			String::from_utf8_lossy(&out.stderr).trim()
		));
	}

	Ok(out.stdout)
}

fn missing_rsync(reason: &str) -> miette::Report {
	miette!(
		help = format!("{PROGRAM} downloads the dump, it comes with the nix dev shell"),
		"failed to execute {PROGRAM}\n{reason}"
	)
}

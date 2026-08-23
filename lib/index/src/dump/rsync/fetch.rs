use std::{fs, path::Path};

use indicatif::ProgressBar;

use hmerr::ioe;

use super::{
	super::super::{keep, progress},
	HOST, PROGRAM, ran,
};

const MARKER: &str = "LATEST";

pub(crate) fn pull(url: &str, into: &Path, bar: &ProgressBar) -> hmerr::Result<()> {
	prepare(into)?;

	progress::rsync(
		PROGRAM,
		&[
			"--archive",
			"--partial",
			"--info=progress2",
			"--no-inc-recursive",
			url,
			&into.to_string_lossy(),
		],
		bar,
	)
}

pub(crate) fn latest_marker(module: &str, into: &Path) -> hmerr::Result<String> {
	let marker = into.join(MARKER);
	small(&format!("{HOST}/{module}/{MARKER}"), &marker)?;

	let name = fs::read_to_string(&marker).map_err(|e| ioe!(marker.to_string_lossy(), e))?;
	forget(into, &[MARKER])?;

	Ok(name.trim().to_string())
}

pub(super) fn small(url: &str, into: &Path) -> hmerr::Result<()> {
	prepare(into)?;
	ran(&["--quiet", url, &into.to_string_lossy()], "fetch", url)?;

	Ok(())
}

pub(super) fn forget(dir: &Path, name: &[&str]) -> hmerr::Result<()> {
	for name in name {
		keep::discard(&dir.join(name))?;
	}

	Ok(())
}

fn prepare(into: &Path) -> hmerr::Result<()> {
	let Some(dir) = into
		.parent()
		.filter(|parent| !parent.as_os_str().is_empty())
	else {
		return Ok(());
	};

	fs::create_dir_all(dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(())
}

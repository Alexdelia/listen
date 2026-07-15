use std::{
	path::{Path, PathBuf},
	process::Command,
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use crate::{declaration::Source, library};

const PLAYER: &str = "mpc";

pub(super) struct Player {
	root: Option<PathBuf>,
}

impl Player {
	pub(super) fn new() -> Self {
		Self { root: None }
	}

	pub(super) fn play(&mut self, mbid: Source) -> hmerr::Result<()> {
		let uri = self.uri(mbid)?;

		mpc(&["insert", &uri])?;
		mpc(&["next"])?;
		mpc(&["play"])
	}

	fn uri(&mut self, mbid: Source) -> hmerr::Result<String> {
		let path = library::recording::path(mbid);
		let path = path
			.canonicalize()
			.map_err(|e| ioe!(path.to_string_lossy(), e))?;

		if let Some(root) = &self.root {
			return relative(root, &path);
		}

		for root in path.ancestors().skip(1) {
			let Ok(uri) = relative(root, &path) else {
				continue;
			};

			if indexed(&uri) {
				self.root = Some(root.to_path_buf());
				return Ok(uri);
			}
		}

		Err(ge!(format!("{R}{PLAYER} does not index {B}{mbid}{D}")).into())
	}
}

fn relative(root: &Path, path: &Path) -> hmerr::Result<String> {
	Ok(path
		.strip_prefix(root)
		.map_err(|e| ge!(format!("{path:?} not under {root:?}\n{e}")))?
		.to_string_lossy()
		.into_owned())
}

fn indexed(uri: &str) -> bool {
	Command::new(PLAYER)
		.arg("listall")
		.arg(uri)
		.output()
		.is_ok_and(|output| output.status.success())
}

fn mpc(arg: &[&str]) -> hmerr::Result<()> {
	let output = Command::new(PLAYER)
		.args(arg)
		.output()
		.map_err(|e| ioe!(PLAYER, e))?;

	if !output.status.success() {
		return Err(ge!(format!(
			"{R}{PLAYER} {arg}{D}\n{err}",
			arg = arg.join(" "),
			err = String::from_utf8_lossy(&output.stderr),
		))
		.into());
	}

	Ok(())
}

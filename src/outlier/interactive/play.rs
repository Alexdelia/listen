use std::{
	path::{Path, PathBuf},
	process::Command,
};

use miette::{IntoDiagnostic, WrapErr, miette};

use crate::{declaration::Source, library};

const PLAYER: &str = "mpc";

pub(super) struct Player {
	root: Option<PathBuf>,
}

impl Player {
	pub(super) const fn new() -> Self {
		Self { root: None }
	}

	pub(super) fn play(&mut self, mbid: Source) -> miette::Result<()> {
		let uri = self.uri(mbid)?;

		mpc(&["insert", &uri])?;
		mpc(&["next"])?;
		mpc(&["play"])
	}

	fn uri(&mut self, mbid: Source) -> miette::Result<String> {
		let path = library::recording::path(mbid);
		let path = path
			.canonicalize()
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot resolve {}", path.display()))?;

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

		Err(miette!("{PLAYER} does not index {mbid}"))
	}
}

fn relative(root: &Path, path: &Path) -> miette::Result<String> {
	Ok(path
		.strip_prefix(root)
		.into_diagnostic()
		.wrap_err_with(|| format!("{} not under {}", path.display(), root.display()))?
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

fn mpc(arg: &[&str]) -> miette::Result<()> {
	let output = Command::new(PLAYER)
		.args(arg)
		.output()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {PLAYER}"))?;

	if !output.status.success() {
		return Err(miette!(
			"{PLAYER} {arg}\n{err}",
			arg = arg.join(" "),
			err = String::from_utf8_lossy(&output.stderr)
		));
	}

	Ok(())
}

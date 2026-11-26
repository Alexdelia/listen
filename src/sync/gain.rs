use std::{
	fs,
	num::NonZeroUsize,
	path::{Path, PathBuf},
	process::Command,
	sync::Mutex,
	thread, vec,
};

use miette::{IntoDiagnostic, WrapErr, bail, miette};

const PROGRAM: &str = "rsgain";
const REPLAY_GAIN_2_REFERENCE: &str = "--loudness=-18";

pub(super) fn ready() -> miette::Result<()> {
	Command::new(PROGRAM)
		.arg("--version")
		.output()
		.map(drop)
		.map_err(|e| {
			miette!(
				help = "enter the dev shell (nix develop, or direnv reload), which provides it",
				"cannot run {PROGRAM}\n{e}"
			)
		})
}

pub(super) fn write(path: &Path) -> miette::Result<()> {
	let hidden = hidden_sibling(path)?;

	fs::copy(path, &hidden)
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to copy {} to {}", path.display(), hidden.display()))?;

	if let Err(e) = tag(&hidden) {
		fs::remove_file(&hidden).ok();
		return Err(e.wrap_err(format!(
			"failed to write the replaygain of {}",
			path.display()
		)));
	}

	fs::rename(&hidden, path)
		.into_diagnostic()
		.wrap_err_with(|| {
			format!(
				"failed to move {} onto {}",
				hidden.display(),
				path.display()
			)
		})
}

fn hidden_sibling(path: &Path) -> miette::Result<PathBuf> {
	let Some(name) = path.file_name() else {
		bail!("{} names no file", path.display());
	};

	Ok(path.with_file_name(format!(".{}", name.to_string_lossy())))
}

fn tag(file: &Path) -> miette::Result<()> {
	let output = Command::new(PROGRAM)
		.args(["custom", "--tagmode=i", REPLAY_GAIN_2_REFERENCE, "--quiet"])
		.arg(file)
		.output()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to execute {PROGRAM}"))?;

	if output.status.success() {
		return Ok(());
	}

	Err(miette!(
		"{PROGRAM} failed on {file}\n{stderr}",
		file = file.display(),
		stderr = console::strip_ansi_codes(&String::from_utf8_lossy(&output.stderr)).trim()
	))
}

pub(super) fn each(path: Vec<PathBuf>, done: impl Fn(miette::Result<()>) + Sync) {
	let queue = Mutex::new(path.into_iter());

	thread::scope(|scope| {
		for _ in 0..worker() {
			scope.spawn(|| {
				while let Some(path) = next(&queue) {
					done(write(&path));
				}
			});
		}
	});
}

fn next(queue: &Mutex<vec::IntoIter<PathBuf>>) -> Option<PathBuf> {
	queue.lock().ok()?.next()
}

fn worker() -> usize {
	thread::available_parallelism().map_or(1, NonZeroUsize::get)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_copy_rsgain_works_on_is_hidden_beside_the_recording() {
		assert_eq!(
			hidden_sibling(Path::new(
				"./output/recording/9ccc0dc3-f639-4525-8b5b-9fdbca7b3f05.mp3"
			))
			.unwrap(),
			Path::new("./output/recording/.9ccc0dc3-f639-4525-8b5b-9fdbca7b3f05.mp3")
		);
	}
}

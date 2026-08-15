use std::{fs, path::Path};

use ansi::abbrev::{B, D, F, Y};
use hmerr::ioe;

use crate::env;

use super::progress;

pub(super) fn requested() -> bool {
	if cfg!(test) {
		return by_test();
	}

	env::get_bool(env::Var::Keep)
}

#[cfg(not(test))]
fn by_test() -> bool {
	false
}

#[cfg(test)]
thread_local! {
	static REQUESTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn by_test() -> bool {
	REQUESTED.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(super) struct Requested;

#[cfg(test)]
impl Drop for Requested {
	fn drop(&mut self) {
		REQUESTED.with(|requested| requested.set(false));
	}
}

#[cfg(test)]
#[must_use]
pub(super) fn request() -> Requested {
	REQUESTED.with(|requested| requested.set(true));

	Requested
}

pub(super) fn discard(path: &Path) -> hmerr::Result<()> {
	if !path.exists() {
		return Ok(());
	}

	if requested() {
		announce(path);
		return Ok(());
	}

	if path.is_dir() {
		fs::remove_dir_all(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	} else {
		fs::remove_file(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;
	}

	Ok(())
}

fn announce(path: &Path) {
	progress::say(format!(
		"{F}{key} is set, keeping {B}{Y}{path}{D}",
		key = env::Var::Keep.key(),
		path = path.display()
	));
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::*;

	fn file(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_keep_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let _ = fs::create_dir_all(&dir);
		let file = dir.join("payload");
		let _ = fs::write(&file, b"payload");

		file
	}

	#[test]
	fn what_the_environment_asks_to_keep_does_not_reach_a_test() {
		assert!(!requested());
	}

	#[test]
	fn discarding_removes_what_it_is_given() {
		let file = file("discard");

		assert!(discard(&file).is_ok());
		assert!(!file.exists());
		let _ = fs::remove_dir_all(file.parent().unwrap_or(&file));
	}

	#[test]
	fn a_requested_keep_leaves_what_it_is_given() {
		let file = file("requested");
		let _keep = request();

		assert!(discard(&file).is_ok());
		assert!(file.exists());
		let _ = fs::remove_dir_all(file.parent().unwrap_or(&file));
	}

	#[test]
	fn a_keep_lasts_no_longer_than_whoever_asked_for_it() {
		{
			let _keep = request();

			assert!(requested());
		}

		assert!(!requested());
	}
}

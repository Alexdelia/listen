pub mod json;
pub mod text;
pub mod username;

use std::{
	ffi::OsString,
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

const APP: &str = "declarative_listen";

const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
const HOME: &str = "HOME";

pub fn root() -> hmerr::Result<PathBuf> {
	Ok(cache_home()?.join(APP))
}

pub fn path(subdir: &str, name: &str, ext: &str) -> hmerr::Result<PathBuf> {
	Ok(root()?.join(subdir).join(name).with_extension(ext))
}

fn cache_home() -> hmerr::Result<PathBuf> {
	if let Some(dir) = non_empty(std::env::var_os(XDG_CACHE_HOME)) {
		let dir = PathBuf::from(dir);
		if dir.is_absolute() {
			return Ok(dir);
		}
	}

	let Some(home) = non_empty(std::env::var_os(HOME)) else {
		return Err(ge!(
			format!("{R}cannot locate a cache directory{D}"),
			h: format!("set {B}{XDG_CACHE_HOME}{D} or {B}{HOME}{D}")
		)
		.into());
	};

	Ok(PathBuf::from(home).join(".cache"))
}

fn non_empty(var: Option<OsString>) -> Option<OsString> {
	var.filter(|v| !v.is_empty())
}

pub fn prepare(path: &Path) -> hmerr::Result<()> {
	let Some(parent) = path.parent() else {
		return Ok(());
	};

	fs::create_dir_all(parent).map_err(|e| ioe!(parent.to_string_lossy(), e))?;

	Ok(())
}

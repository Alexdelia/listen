pub mod flag;
pub mod json;
pub mod text;
pub mod username;

use std::{
	ffi::OsString,
	fs,
	path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, WrapErr, miette};

const APP: &str = "declarative_listen";

const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
const HOME: &str = "HOME";

pub fn root() -> miette::Result<PathBuf> {
	Ok(cache_home()?.join(APP))
}

pub fn path(subdir: &str, name: &str, ext: &str) -> miette::Result<PathBuf> {
	Ok(root()?.join(subdir).join(name).with_extension(ext))
}

fn cache_home() -> miette::Result<PathBuf> {
	if let Some(dir) = non_empty(std::env::var_os(XDG_CACHE_HOME)) {
		let dir = PathBuf::from(dir);
		if dir.is_absolute() {
			return Ok(dir);
		}
	}

	let Some(home) = non_empty(std::env::var_os(HOME)) else {
		return Err(miette!(
			help = format!("set {XDG_CACHE_HOME} or {HOME}"),
			"cannot locate a cache directory"
		));
	};

	Ok(PathBuf::from(home).join(".cache"))
}

fn non_empty(var: Option<OsString>) -> Option<OsString> {
	var.filter(|v| !v.is_empty())
}

pub fn prepare(path: &Path) -> miette::Result<()> {
	let Some(parent) = path.parent() else {
		return Ok(());
	};

	fs::create_dir_all(parent)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot create {}", parent.display()))?;

	Ok(())
}

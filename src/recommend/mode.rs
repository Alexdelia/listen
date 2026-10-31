use std::path::{Path, PathBuf};

use super::{
	local::{self, Local},
	log::Log,
	target::{self, Target},
};

pub(super) enum Mode {
	Interactive,
	Trace { limit: usize },
}

impl Mode {
	pub(super) const fn of(trace: Option<usize>) -> Self {
		match trace {
			Some(limit) => Self::Trace { limit },
			None => Self::Interactive,
		}
	}

	pub(super) fn log(
		&self,
		path: impl FnOnce() -> miette::Result<PathBuf>,
	) -> miette::Result<Log> {
		match self {
			Self::Interactive => Ok(Log::File(path()?)),
			Self::Trace { .. } => Ok(Log::Off),
		}
	}

	pub(super) fn open(&self, path: &Path) -> miette::Result<Local> {
		match self {
			Self::Interactive => local::open(path),
			Self::Trace { .. } => local::standing(path),
		}
	}

	pub(super) fn target(&self, target: Option<&str>) -> miette::Result<Target> {
		match self {
			Self::Interactive => target::resolve(target),
			Self::Trace { .. } => target::known(target),
		}
	}
}

mod decline;
mod fetch;
mod find;
#[cfg(test)]
mod fixture;
mod published;
mod release;

use std::path::PathBuf;

use miette::miette;

pub(super) use decline::{decline, declined};
pub(super) use fetch::{fetch, fetch_named};
pub(super) use find::find;
pub(super) use published::{PREFIX, newer_than};
pub(super) use release::discard;

const LISTEN: &str = "listen";

pub(crate) struct Listen {
	pub dir: PathBuf,
	pub name: String,
}

pub(super) struct Offer {
	pub reason: &'static str,
	pub default: bool,
}

pub(super) fn refused() -> miette::Report {
	miette!(
		help = "the index is built from the dump, no dump means nothing to recommend from",
		"cancelled"
	)
}

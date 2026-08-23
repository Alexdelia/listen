mod bar;
mod child;
mod eta;
mod screen;

use indicatif::HumanBytes;

use super::decide::Decide;

pub(super) use bar::{Measure, started, waiting_bar};
pub(super) use child::{complaint, rsync};
pub(super) use screen::{ended, say};

pub(super) fn bytes(size: u64) -> String {
	HumanBytes(size).to_string()
}

pub(super) fn confirm(decide: &dyn Decide, question: &str, default: bool) -> hmerr::Result<bool> {
	screen::suspended(|| decide.confirm(question, default))
}

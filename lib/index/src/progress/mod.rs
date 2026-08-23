mod bar;
mod child;
mod eta;
mod screen;

use indicatif::HumanBytes;

pub(super) use bar::{Measure, started, waiting_bar};
pub(super) use child::{complaint, rsync};
pub(super) use screen::{ask, ended, say};

pub(super) fn bytes(size: u64) -> String {
	HumanBytes(size).to_string()
}

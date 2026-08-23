mod bar;
mod board;
mod child;
mod eta;

use indicatif::HumanBytes;

pub(super) use bar::{Measure, started, waiting_bar};
pub(super) use board::{ask, ended, say};
pub(super) use child::{complaint, rsync};

pub(super) fn bytes(size: u64) -> String {
	HumanBytes(size).to_string()
}

use std::ops::Deref;

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use indicatif::ProgressBar;

use super::super::{open::BUCKET, progress};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
	Library,
	Compact,
	Recording,
	Own,
	Pool,
	Artist,
	Credit,
	Stat,
	UserStat,
	Listen,
}

const PLAN: [Stage; 10] = [
	Stage::Library,
	Stage::Compact,
	Stage::Recording,
	Stage::Own,
	Stage::Pool,
	Stage::Artist,
	Stage::Credit,
	Stage::Stat,
	Stage::UserStat,
	Stage::Listen,
];

const ONCE: u64 = 1;

impl Stage {
	pub(super) fn title(self) -> &'static str {
		match self {
			Self::Library => "library",
			Self::Compact => "compact",
			Self::Recording => "recording",
			Self::Own => "own",
			Self::Pool => "pool",
			Self::Artist => "artist",
			Self::Credit => "credit",
			Self::Stat => "stat",
			Self::UserStat => "user stat",
			Self::Listen => "listen",
		}
	}

	fn total(self, batch: u64) -> u64 {
		match self {
			Self::Library | Self::Artist => batch,
			Self::Compact | Self::Stat | Self::Listen => u64::from(BUCKET),
			Self::Recording | Self::Own | Self::Pool | Self::Credit | Self::UserStat => ONCE,
		}
	}
}

pub(super) struct Board {
	bar: Vec<ProgressBar>,
}

pub(crate) struct Running {
	bar: ProgressBar,
}

impl Board {
	pub(super) fn of(batch: usize) -> hmerr::Result<Self> {
		let batch = u64::try_from(batch).unwrap_or(ONCE);
		let mut bar = Vec::with_capacity(PLAN.len());

		for stage in PLAN {
			bar.push(progress::waiting_bar(stage.total(batch), stage.title())?);
		}

		Ok(Self { bar })
	}

	pub(super) fn start(&self, stage: Stage) -> hmerr::Result<Running> {
		let bar = PLAN
			.iter()
			.position(|planned| *planned == stage)
			.and_then(|at| self.bar.get(at))
			.ok_or_else(|| {
				ge!(format!(
					"{R}no bar was planned for {B}{title}{D}",
					title = stage.title()
				))
			})?;

		progress::started(bar, stage.title())?;

		Ok(Running { bar: bar.clone() })
	}
}

impl Deref for Running {
	type Target = ProgressBar;

	fn deref(&self) -> &Self::Target {
		&self.bar
	}
}

impl Drop for Running {
	fn drop(&mut self) {
		progress::ended(&self.bar);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_stage_of_a_build_is_on_the_board_before_the_first_one_runs() {
		let board = Board::of(48).unwrap_or_else(|_| unreachable!());

		assert_eq!(board.bar.len(), PLAN.len());
		for stage in PLAN {
			assert!(board.start(stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn what_a_stage_is_worth_is_read_off_how_the_dump_was_sliced() {
		assert_eq!(Stage::Library.total(48), 48);
		assert_eq!(Stage::Artist.total(48), 48);
		assert_eq!(Stage::Listen.total(48), u64::from(BUCKET));
		assert_eq!(Stage::Pool.total(48), ONCE);
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in PLAN {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

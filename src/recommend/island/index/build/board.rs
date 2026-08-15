use super::super::{
	board::{Board, Running},
	progress::Measure,
};

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
			Self::Compact | Self::Stat | Self::Listen => u64::from(super::super::open::BUCKET),
			Self::Recording | Self::Own | Self::Pool | Self::Credit | Self::UserStat => ONCE,
		}
	}
}

pub(super) fn of(batch: usize) -> hmerr::Result<Board> {
	let batch = u64::try_from(batch).unwrap_or(ONCE);

	Board::of(&PLAN.map(|stage| (stage.title(), Measure::Step(stage.total(batch)))))
}

pub(super) fn start(board: &Board, stage: Stage) -> hmerr::Result<Running> {
	board.start(stage.title())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_stage_of_a_build_is_on_the_board_before_the_first_one_runs() {
		let board = of(48).unwrap_or_else(|_| unreachable!());

		for stage in PLAN {
			assert!(start(&board, stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn what_a_stage_is_worth_is_read_off_how_the_dump_was_sliced() {
		assert_eq!(Stage::Library.total(48), 48);
		assert_eq!(Stage::Artist.total(48), 48);
		assert_eq!(
			Stage::Listen.total(48),
			u64::from(super::super::super::open::BUCKET)
		);
		assert_eq!(Stage::Pool.total(48), ONCE);
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in PLAN {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

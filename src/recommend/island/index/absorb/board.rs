use super::super::{
	board::{Board, Running},
	open::BUCKET,
	progress::Measure,
};

#[derive(Clone, Copy)]
pub(super) enum Stage {
	Recording,
	Credit,
	Listen,
	Stat,
	UserStat,
}

const PLAN: [Stage; 5] = [
	Stage::Recording,
	Stage::Credit,
	Stage::Listen,
	Stage::Stat,
	Stage::UserStat,
];

const ONCE: u64 = 1;

impl Stage {
	pub(super) fn title(self) -> &'static str {
		match self {
			Self::Recording => "recording",
			Self::Credit => "credit",
			Self::Listen => "listen",
			Self::Stat => "stat",
			Self::UserStat => "user stat",
		}
	}

	fn total(self) -> u64 {
		match self {
			Self::Listen | Self::Stat => u64::from(BUCKET),
			Self::Recording | Self::Credit | Self::UserStat => ONCE,
		}
	}
}

pub(super) fn of() -> hmerr::Result<Board> {
	Board::of(&PLAN.map(|stage| (stage.title(), Measure::Step(stage.total()))))
}

pub(super) fn start(board: &Board, stage: Stage) -> hmerr::Result<Running> {
	board.start(stage.title())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_stage_of_a_merge_is_on_the_board_before_the_first_one_runs() {
		let board = of().unwrap_or_else(|_| unreachable!());

		for stage in PLAN {
			assert!(start(&board, stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn a_bucketed_stage_is_worth_one_step_per_bucket() {
		assert_eq!(Stage::Listen.total(), u64::from(BUCKET));
		assert_eq!(Stage::Stat.total(), u64::from(BUCKET));
		assert_eq!(Stage::Recording.total(), ONCE);
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in PLAN {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

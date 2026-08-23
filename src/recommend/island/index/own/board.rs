use super::super::{
	board::{Board, Running},
	dump::{self, Pending},
	progress::Measure,
};

#[derive(Clone, Copy)]
pub(super) enum Stage {
	Download,
	Verify,
	Unpack,
	Listen,
}

const PLAN: [Stage; 4] = [Stage::Download, Stage::Verify, Stage::Unpack, Stage::Listen];

pub(super) struct Chain {
	pub dump: u64,
	pub byte: u64,
}

impl Stage {
	pub(super) fn title(self) -> &'static str {
		match self {
			Self::Download => "download",
			Self::Verify => "verify",
			Self::Unpack => "unpack",
			Self::Listen => "listen",
		}
	}

	fn measure(self, chain: &Chain) -> Measure {
		match self {
			Self::Download | Self::Unpack => Measure::Byte(chain.byte),
			Self::Verify | Self::Listen => Measure::Step(chain.dump),
		}
	}
}

pub(super) fn chain(pending: &[&Pending]) -> Chain {
	Chain {
		dump: u64::try_from(pending.len()).unwrap_or_default(),
		byte: dump::weight(pending),
	}
}

pub(super) fn of(chain: &Chain) -> hmerr::Result<Board> {
	Board::of(&PLAN.map(|stage| (stage.title(), stage.measure(chain))))
}

pub(super) fn start(board: &Board, stage: Stage) -> hmerr::Result<Running> {
	board.start(stage.title())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn chain() -> Chain {
		Chain {
			dump: 30,
			byte: 6 << 30,
		}
	}

	#[test]
	fn the_whole_fold_is_on_one_board_before_the_first_byte_is_fetched() {
		let board = of(&chain()).unwrap_or_else(|_| unreachable!());

		for stage in PLAN {
			assert!(start(&board, stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn a_fetch_stage_is_measured_over_every_pending_dump_not_one() {
		let chain = chain();

		assert!(matches!(
			Stage::Download.measure(&chain),
			Measure::Byte(byte) if byte == chain.byte
		));
		assert!(matches!(
			Stage::Listen.measure(&chain),
			Measure::Step(step) if step == chain.dump
		));
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in PLAN {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

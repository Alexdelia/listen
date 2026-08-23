use super::super::{board::Planned, index::layout::BUCKET, progress::Measure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
	Download,
	Verify,
	Unpack,
	Fold,
	Recording,
	Credit,
	Listen,
	Listener,
	Stat,
	UserStat,
}

pub(super) const PLAN: [Stage; 10] = [
	Stage::Download,
	Stage::Verify,
	Stage::Unpack,
	Stage::Fold,
	Stage::Recording,
	Stage::Credit,
	Stage::Listen,
	Stage::Listener,
	Stage::Stat,
	Stage::UserStat,
];

const ONCE: u64 = 1;

pub(super) struct Chain {
	pub dump: u64,
	pub byte: u64,
}

impl Planned for Stage {
	type Cost = Chain;

	fn title(self) -> &'static str {
		match self {
			Self::Download => "download",
			Self::Verify => "verify",
			Self::Unpack => "unpack",
			Self::Fold => "fold",
			Self::Recording => "recording",
			Self::Credit => "credit",
			Self::Listen => "listen",
			Self::Listener => "listener",
			Self::Stat => "stat",
			Self::UserStat => "user stat",
		}
	}

	fn measure(self, chain: &Chain) -> Measure {
		match self {
			Self::Download | Self::Unpack => Measure::Byte(chain.byte),
			Self::Verify | Self::Fold => Measure::Step(chain.dump),
			Self::Listen | Self::Stat => Measure::Step(u64::from(BUCKET)),
			Self::Recording | Self::Credit | Self::Listener | Self::UserStat => Measure::Step(ONCE),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{super::super::board::Board, *};

	fn chain() -> Chain {
		Chain {
			dump: 30,
			byte: 6 << 30,
		}
	}

	#[test]
	fn the_whole_absorb_is_on_one_board_before_the_first_byte_is_fetched() {
		let board = Board::of(&PLAN, &chain()).unwrap_or_else(|_| unreachable!());

		for stage in PLAN {
			assert!(board.start(stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn a_fetch_stage_is_measured_over_the_whole_chain_not_one_dump() {
		let chain = chain();

		assert!(matches!(
			Stage::Download.measure(&chain),
			Measure::Byte(byte) if byte == chain.byte
		));
		assert!(matches!(
			Stage::Fold.measure(&chain),
			Measure::Step(step) if step == chain.dump
		));
	}

	#[test]
	fn a_merge_stage_is_measured_in_buckets_or_in_one_step() {
		let chain = chain();

		assert!(matches!(
			Stage::Listen.measure(&chain),
			Measure::Step(step) if step == u64::from(BUCKET)
		));
		assert!(matches!(
			Stage::Recording.measure(&chain),
			Measure::Step(ONCE)
		));
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in PLAN {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

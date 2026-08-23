use super::super::{board::Planned, progress::Measure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
	Download,
	Verify,
	Unpack,
	Relation,
}

pub(super) const LISTEN: [Stage; 3] = [Stage::Download, Stage::Verify, Stage::Unpack];

pub(super) const MUSIC_BRAINZ: [Stage; 4] = [
	Stage::Download,
	Stage::Verify,
	Stage::Unpack,
	Stage::Relation,
];

const ONCE: u64 = 1;

impl Planned for Stage {
	type Cost = u64;

	fn title(self) -> &'static str {
		match self {
			Self::Download => "download",
			Self::Verify => "verify",
			Self::Unpack => "unpack",
			Self::Relation => "relation",
		}
	}

	fn measure(self, archive: &u64) -> Measure {
		match self {
			Self::Download | Self::Unpack => Measure::Byte(*archive),
			Self::Verify | Self::Relation => Measure::Step(ONCE),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{super::super::board::Board, *};

	#[test]
	fn what_a_dump_costs_is_laid_out_before_the_first_byte_is_fetched() {
		let board = Board::of(&LISTEN, &(1 << 30)).unwrap_or_else(|_| unreachable!());

		for stage in LISTEN {
			assert!(board.start(stage).is_ok(), "{}", stage.title());
		}
	}

	#[test]
	fn only_the_musicbrainz_dump_ends_in_artist_relations() {
		let listen = Board::of(&LISTEN, &(1 << 30)).unwrap_or_else(|_| unreachable!());
		let music_brainz = Board::of(&MUSIC_BRAINZ, &(1 << 30)).unwrap_or_else(|_| unreachable!());

		assert!(listen.start(Stage::Relation).is_err());
		assert!(music_brainz.start(Stage::Relation).is_ok());
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for stage in MUSIC_BRAINZ {
			assert!(stage.title().len() <= 9, "{}", stage.title());
		}
	}
}

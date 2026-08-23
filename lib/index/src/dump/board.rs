use super::super::{board::Board, progress::Measure};

pub(super) const DOWNLOAD: &str = "download";
pub(super) const VERIFY: &str = "verify";
pub(super) const UNPACK: &str = "unpack";
pub(super) const RELATION: &str = "relation";

const ONCE: u64 = 1;

pub(super) fn listen(archive: u64) -> hmerr::Result<Board> {
	Board::of(&[
		(DOWNLOAD, Measure::Byte(archive)),
		(VERIFY, Measure::Step(ONCE)),
		(UNPACK, Measure::Byte(archive)),
	])
}

pub(super) fn music_brainz(archive: u64) -> hmerr::Result<Board> {
	Board::of(&[
		(DOWNLOAD, Measure::Byte(archive)),
		(VERIFY, Measure::Step(ONCE)),
		(UNPACK, Measure::Byte(archive)),
		(RELATION, Measure::Step(ONCE)),
	])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn what_a_dump_costs_is_laid_out_before_the_first_byte_is_fetched() {
		let board = listen(1 << 30).unwrap_or_else(|_| unreachable!());

		for title in [DOWNLOAD, VERIFY, UNPACK] {
			assert!(board.start(title).is_ok(), "{title}");
		}
	}

	#[test]
	fn only_the_musicbrainz_dump_ends_in_artist_relations() {
		let listen = listen(1 << 30).unwrap_or_else(|_| unreachable!());
		let music_brainz = music_brainz(1 << 30).unwrap_or_else(|_| unreachable!());

		assert!(listen.start(RELATION).is_err());
		assert!(music_brainz.start(RELATION).is_ok());
	}

	#[test]
	fn a_title_never_outgrows_the_column_it_is_printed_in() {
		for title in [DOWNLOAD, VERIFY, UNPACK, RELATION] {
			assert!(title.len() <= 9, "{title}");
		}
	}
}

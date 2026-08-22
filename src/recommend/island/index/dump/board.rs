use super::super::{board::Board, progress::Measure};

pub(super) const DOWNLOAD: &str = "download";
pub(super) const VERIFY: &str = "verify";
pub(super) const UNPACK: &str = "unpack";
pub(super) const RELATION: &str = "relation";
pub(super) const FOLD: &str = "fold";

const ONCE: u64 = 1;

pub(super) fn listen(archive: u64) -> hmerr::Result<Board> {
	Board::of(&[
		(DOWNLOAD, Measure::Byte(archive)),
		(VERIFY, Measure::Step(ONCE)),
		(UNPACK, Measure::Byte(archive)),
	])
}

pub(super) fn incremental(archive: u64) -> hmerr::Result<Board> {
	Board::of(&[
		(DOWNLOAD, Measure::Byte(archive)),
		(VERIFY, Measure::Step(ONCE)),
		(UNPACK, Measure::Byte(archive)),
		(FOLD, Measure::Step(ONCE)),
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
	fn an_incremental_dump_ends_in_the_fold_that_keeps_it() {
		let incremental = incremental(1 << 30).unwrap_or_else(|_| unreachable!());

		assert!(incremental.start(FOLD).is_ok());
		assert!(
			listen(1 << 30)
				.unwrap_or_else(|_| unreachable!())
				.start(FOLD)
				.is_err()
		);
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
		for title in [DOWNLOAD, VERIFY, UNPACK, RELATION, FOLD] {
			assert!(title.len() <= 9, "{title}");
		}
	}
}

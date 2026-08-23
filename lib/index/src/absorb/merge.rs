use std::path::Path;

use ansi::abbrev::{B, D, F, G};
use chrono::Utc;

use super::{
	super::{board::Board, index::Meta, progress, query},
	artist, recording, recording_listener,
	stage::Stage,
	user_listen, user_stat,
	work::{self, Reach},
};

pub(super) fn merge(
	db: &duckdb::Connection,
	board: &Board<Stage>,
	dir: &Path,
	work: &Path,
	held: &Meta,
	reach: Reach,
) -> hmerr::Result<()> {
	announce(&reach);

	let merge = work::merging(dir, work, &reach.covered)?;

	let recording = recording::of(db, board, &merge)?;
	artist::of(db, board, &merge, &recording)?;
	let row = user_listen::of(db, board, &merge, &recording)?;
	recording_listener::of(db, board, &merge)?;
	let user = user_stat::of(db, board, &merge)?;

	let meta = Meta {
		built: Utc::now().date_naive().to_string(),
		dump: held.dump.clone(),
		own: held.own,
		reached: Some(reach.covered),
		gap: reach.gap,
		absorbed: held.absorbed + reach.absorbed,
		user,
		recording: query::count(db, &recording)?,
		user_listen: row,
	};

	work::publish(&merge.into, dir, &meta)?;
	work::release(work);

	progress::say(format!(
		"{G}index absorbed up to {B}{to}{D}",
		to = meta.covered()
	));

	Ok(())
}

fn announce(reach: &Reach) {
	progress::say(format!(
		"\n{F}merging {B}{absorbed}{D}{F} absorbed dump into the index{D}\n",
		absorbed = reach.absorbed
	));
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{
		super::{
			super::{board::Chain, index},
			fixture::{
				BUILT, DECLARED, FRESH, LATER, OTHER_RECORDING, POOLED, built, day, following,
				incremental, morrow, one, plays,
			},
			reach::taken,
			stage,
		},
		*,
	};

	#[test]
	fn a_merge_left_half_done_is_redone_over_a_dump_folded_after_it() {
		let (dir, index, meta) = built("resumed");
		let work = work::open(&index, meta.covered()).unwrap_or_default();
		let db = index::session::of(&work).unwrap_or_else(|_| unreachable!());
		let mut reach = work::reach(&work, &meta);
		let board =
			Board::of(&stage::PLAN, &Chain { dump: 2, byte: 0 }).unwrap_or_else(|_| unreachable!());

		taken(&db, &work, &mut reach, &incremental(&dir, BUILT, &day()))
			.unwrap_or_else(|e| unreachable!("{e}"));
		let staged =
			work::merging(&index, &work, &reach.covered).unwrap_or_else(|e| unreachable!("{e}"));
		recording::of(&db, &board, &staged).unwrap_or_else(|e| unreachable!("{e}"));

		taken(&db, &work, &mut reach, &following(&dir, &morrow()))
			.unwrap_or_else(|e| unreachable!("{e}"));
		merge(&db, &board, &index, &work, &meta, reach).unwrap_or_else(|e| unreachable!("{e}"));

		assert_eq!(
			one::<i64>(&index, "select count(*)::bigint from recording"),
			i64::try_from(DECLARED + OTHER_RECORDING + 2).unwrap_or_default(),
			"the dump folded after the merge died reaches the index too"
		);
		assert_eq!(plays(&index, POOLED, FRESH + 1), 3);
		assert_eq!(plays(&index, POOLED, FRESH), 3);
		assert_eq!(
			index::meta::read(&index)
				.unwrap_or_else(|_| unreachable!())
				.covered(),
			LATER
		);
		let _ = fs::remove_dir_all(&dir);
	}
}

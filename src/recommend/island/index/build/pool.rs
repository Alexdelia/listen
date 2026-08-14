use std::path::{Path, PathBuf};

use ansi::abbrev::{B, D, F, R};
use hmerr::{GenericError, ge};

use super::scan::Scan;

const POOL: &str = "user.parquet";

const MIN_SEED_PER_USER: usize = 5;
const MIN_PLAY_PER_SEED: u32 = 3;
const MIN_OWN_COVERAGE_PERCENT: i64 = 50;
const MIN_OWN_MARGIN_PERCENT: i64 = 150;

pub(super) fn of(scan: &Scan, listen: &Path, declared: usize) -> hmerr::Result<PathBuf> {
	let own = own(scan, listen, declared)?;
	let into = scan.work.join(POOL);

	scan.copy(
		&into,
		&format!(
			r"
select user_id
from read_parquet('{listen}')
where plays >= {MIN_PLAY_PER_SEED} and user_id <> {own}
group by user_id
having count(*) >= {MIN_SEED_PER_USER}
",
			listen = listen.display()
		),
	)?;

	Ok(into)
}

fn own(scan: &Scan, listen: &Path, declared: usize) -> hmerr::Result<i64> {
	let mut statement = scan.db.prepare(&format!(
		r"
select user_id::bigint, count(*)::bigint
from read_parquet('{listen}')
group by 1
order by 2 desc
limit 2
",
		listen = listen.display()
	))?;

	let mut row = statement.query([])?;
	let mut top = Vec::new();
	while let Some(row) = row.next()? {
		top.push((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?));
	}

	let Some((own, seed)) = top.first().copied() else {
		return Err(no_overlap().into());
	};
	let runner_up = top.get(1).map_or(0, |(_, seed)| *seed);

	if !separated(seed, runner_up, declared) {
		return Err(ambiguous(own, seed, runner_up).into());
	}

	println!(
		"{F}own listenbrainz user {B}{own}{D}{F}, {B}{share}%{D}{F} of the declared library, \
		runner up at {B}{runner_up}{D}",
		share = coverage(seed, declared)
	);

	Ok(own)
}

fn coverage(seed: i64, declared: usize) -> i64 {
	let declared = i64::try_from(declared).unwrap_or(i64::MAX).max(1);

	100 * seed / declared
}

fn separated(seed: i64, runner_up: i64, declared: usize) -> bool {
	coverage(seed, declared) >= MIN_OWN_COVERAGE_PERCENT
		&& 100 * seed >= MIN_OWN_MARGIN_PERCENT * runner_up
}

fn no_overlap() -> GenericError {
	ge!(
		format!("{R}no listener in the dump has played any declared recording{D}"),
		h: "the dump may be incomplete, check that every parquet shard extracted"
	)
}

fn ambiguous(own: i64, seed: i64, runner_up: i64) -> GenericError {
	ge!(
		format!(
			"{R}cannot tell which listenbrainz user is yours: {B}{own}{D}{R} has {B}{seed}{D}{R} \
			declared recordings against {B}{runner_up}{D}{R} for the next one{D}"
		),
		h: "the index must exclude your own listens, else it recommends what you already declared"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_runner_of_the_library_is_separated_from_a_mere_fan() {
		assert!(separated(815, 426, 1019));
	}

	#[test]
	fn a_thin_lead_is_not_separation() {
		assert!(!separated(600, 590, 1019));
	}

	#[test]
	fn covering_little_of_the_library_is_not_separation_however_big_the_lead() {
		assert!(!separated(100, 1, 1019));
	}

	#[test]
	fn a_lone_listener_covering_the_library_is_separated() {
		assert!(separated(1019, 0, 1019));
	}

	#[test]
	fn coverage_is_a_percentage_of_what_is_declared() {
		assert_eq!(coverage(815, 1019), 79);
		assert_eq!(coverage(1019, 1019), 100);
	}

	#[test]
	fn an_empty_declaration_cannot_divide_by_zero() {
		assert_eq!(coverage(0, 0), 0);
	}
}

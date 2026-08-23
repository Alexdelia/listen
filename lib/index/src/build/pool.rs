use std::path::{Path, PathBuf};

use ansi::abbrev::{B, D, F, R};
use hmerr::{GenericError, ge};

use super::{super::progress, library, scan::Scan, seed, stage::Stage};

const POOL: &str = "user.parquet";

const MIN_PLAY_PER_RECORDING: u32 = 3;
const MIN_REPEATED_RECORDING: usize = 20;
const MIN_OWN_COVERAGE_PERCENT: i64 = 50;
const MIN_OWN_MARGIN_PERCENT: i64 = 150;

pub(super) struct Pool {
	pub path: PathBuf,
	pub own: u32,
}

#[derive(Clone, Copy)]
pub(super) struct Listener {
	pub named: Option<u32>,
	pub known: Option<u32>,
}

impl Pool {
	pub(super) fn read(&self) -> String {
		format!("read_parquet('{path}')", path = self.path.display())
	}
}

pub(super) fn of(
	scan: &Scan,
	library: &Path,
	declared: usize,
	listener: Listener,
) -> hmerr::Result<Pool> {
	let own = own(scan, library, declared, listener)?;
	let path = scan.work.join(POOL);

	scan.step(
		Stage::Pool,
		&path,
		&format!(
			r"
select user_id
from {library}
where plays >= {MIN_PLAY_PER_RECORDING} and user_id <> {own}
group by user_id
having count(*) >= {MIN_REPEATED_RECORDING}
",
			library = library::read(library)
		),
	)?;

	Ok(Pool { path, own })
}

fn own(scan: &Scan, library: &Path, declared: usize, listener: Listener) -> hmerr::Result<u32> {
	if let Some(named) = listener.named {
		progress::say(format!(
			"{F}own listenbrainz user {B}{named}{D}{F}: what the listens dump names{D}"
		));

		return Ok(named);
	}

	let top = seeded(scan, library)?;

	let Some((own, seed)) = top.first().copied() else {
		return Err(no_overlap().into());
	};
	let runner_up = top.get(1).map_or(0, |(_, seed)| *seed);

	if separated(seed, runner_up, declared) {
		progress::say(format!(
			"{F}own listenbrainz user {B}{own}{D}{F}: {B}{share}%{D}{F} of declared library, \
			runner up {B}{runner_up}{D}",
			share = coverage(seed, declared)
		));

		return Ok(own);
	}

	let Some(known) = listener.known else {
		return Err(ambiguous(own, seed, runner_up).into());
	};

	progress::say(format!(
		"{F}declaration no longer singles out a listener, keeping known own user {B}{known}{D}"
	));

	Ok(known)
}

fn seeded(scan: &Scan, library: &Path) -> hmerr::Result<Vec<(u32, i64)>> {
	let bar = scan.stage(Stage::Own)?;
	let db = scan.take();

	let mut statement = db.prepare(&format!(
		r"
select l.user_id::uinteger, count(*)::bigint
from {library} l
semi join {seed} s on s.mbid = l.mbid
group by 1
order by 2 desc
limit 2
",
		library = library::read(library),
		seed = seed::NAME
	))?;

	let mut row = statement.query([])?;
	let mut top = Vec::new();
	while let Some(row) = row.next()? {
		top.push((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?));
	}

	bar.inc(1);

	Ok(top)
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

fn ambiguous(own: u32, seed: i64, runner_up: i64) -> GenericError {
	ge!(
		format!(
			"{R}cannot tell which listenbrainz user is yours: {B}{own}{D}{R} has {B}{seed}{D}{R} \
			declared recording, next one {B}{runner_up}{D}"
		),
		h: "the index must exclude your own listen, else it recommends what you declared"
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

mod backtest;
mod calibrate;
mod diversify;
#[cfg(test)]
mod fixture;
mod log;
mod predict;
mod preference;
mod profile;
mod rated;
mod report;

use std::path::PathBuf;

use crate::{
	args::IslandArg,
	declaration::{Entry, Q, value},
};

use super::{
	declared,
	feed::{self, Fed},
	island, known_artist,
	local::Local,
	log::Log,
};

use calibrate::Calibration;
use diversify::Pick;
use predict::{Point, Scored};

pub(super) use report::Report;

const POOL: usize = 2000;

pub(super) fn log_path() -> hmerr::Result<PathBuf> {
	log::path()
}

pub(super) fn feed(local: &Local, arg: &IslandArg, log: Log) -> hmerr::Result<Fed> {
	let db = &local.index.db;
	prepare(db, &local.entry, arg.allow_known_artist)?;
	let (calibration, point) = calibrated(db, &local.entry)?;

	let mut judged: Vec<(f32, Scored)> = predict::candidates(db)?
		.into_iter()
		.map(|scored| (calibration.expected(scored.raw), scored))
		.collect();
	judged.sort_by(|a, b| b.0.total_cmp(&a.0));

	let gate = f32::from(value::NEUTRAL);
	let passing = judged
		.iter()
		.take_while(|(expected, _)| *expected >= gate)
		.count();

	let report = report::of(
		calibration,
		&point,
		arg.allow_known_artist,
		&judged,
		passing,
	);

	judged.truncate(passing.min(POOL));
	let pick = picked(db, judged)?;

	Ok(Fed {
		feed: Box::new(diversify::stream(pick, arg.allow_known_artist, log)),
		report: feed::Report::Forecast(report),
	})
}

fn picked(db: &duckdb::Connection, judged: Vec<(f32, Scored)>) -> hmerr::Result<Vec<Pick>> {
	if judged.is_empty() {
		return Ok(Vec::new());
	}

	let pool: Vec<Scored> = judged.iter().map(|(_, scored)| *scored).collect();
	let mut shape = profile::of(db, &pool)?;

	Ok(judged
		.into_iter()
		.map(|(expected, scored)| {
			let shape = shape
				.remove(&scored.recording_id)
				.unwrap_or_else(|| profile::Shape {
					profile: profile::Profile::default(),
					near: Vec::new(),
				});

			Pick {
				mbid: scored.mbid,
				raw: scored.raw,
				expected,
				support: scored.support,
				profile: shape.profile,
				near: shape.near,
			}
		})
		.collect())
}

pub(super) fn backtest(local: &Local) -> hmerr::Result<()> {
	let db = &local.index.db;
	prepare(db, &local.entry, false)?;
	let (calibration, point) = calibrated(db, &local.entry)?;

	backtest::print(&paired(&point), calibration);

	Ok(())
}

fn prepare(
	db: &duckdb::Connection,
	entry: &[Entry],
	allow_known_artist: bool,
) -> hmerr::Result<()> {
	declared::table(db, entry)?;
	preference::declare(db)?;
	known_artist::declare(db, allow_known_artist)?;
	predict::prepare(db)?;

	Ok(())
}

fn calibrated(
	db: &duckdb::Connection,
	entry: &[Entry],
) -> hmerr::Result<(Calibration, Vec<Point>)> {
	let rated = rated::of(entry, &[island::log_path()?, log::path()?])?;
	let point = predict::rated(db, &rated)?;

	Ok((Calibration::fit(&paired(&point)), point))
}

fn paired(point: &[Point]) -> Vec<(Q, f32)> {
	point.iter().map(|point| (point.q, point.raw)).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_empty_pool_asks_the_index_nothing() {
		let db = duckdb::Connection::open_in_memory().unwrap();

		assert!(picked(&db, Vec::new()).is_ok_and(|pick| pick.is_empty()));
	}
}

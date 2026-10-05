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

use ansi::abbrev::{B, D, F, Y};

use crate::{
	args::IslandArg,
	declaration::{Entry, Q, value},
	format::human_readable_number,
};

use super::{declared, feed::Feed, island, known_artist, local::Local};

use calibrate::Calibration;
use diversify::Pick;
use predict::Scored;

const POOL: usize = 2000;

pub(super) fn feed(local: &Local, arg: &IslandArg) -> hmerr::Result<Box<dyn Feed>> {
	let db = &local.index.db;
	prepare(db, &local.entry, arg.allow_known_artist)?;
	let (calibration, _) = calibrated(db, &local.entry)?;

	let candidate = predict::candidates(db)?;
	let total = candidate.len();

	let mut judged: Vec<(f32, Scored)> = candidate
		.into_iter()
		.map(|scored| (calibration.expected(scored.raw), scored))
		.collect();
	judged.sort_by(|a, b| b.0.total_cmp(&a.0));

	let gate = f32::from(value::NEUTRAL);
	let passing = judged
		.iter()
		.take_while(|(expected, _)| *expected >= gate)
		.count();

	if passing == 0 {
		nothing(judged.first());
		return Ok(Box::new(diversify::stream(
			Vec::new(),
			arg.allow_known_artist,
			log::path()?,
		)));
	}

	println!(
		"{B}similar{D} {Y}{passing}{D} {F}of{D} {total} {F}candidates reach q1 ({calibration}){D}",
		total = human_readable_number::text(u64::try_from(total).unwrap_or(u64::MAX)),
	);

	judged.truncate(passing.min(POOL));
	let pool: Vec<Scored> = judged.iter().map(|(_, scored)| *scored).collect();
	let mut shape = profile::of(db, &pool)?;

	let pick = judged
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
		.collect();

	Ok(Box::new(diversify::stream(
		pick,
		arg.allow_known_artist,
		log::path()?,
	)))
}

pub(super) fn backtest(local: &Local) -> hmerr::Result<()> {
	let db = &local.index.db;
	prepare(db, &local.entry, false)?;
	let (calibration, point) = calibrated(db, &local.entry)?;

	backtest::print(&point, calibration);

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
) -> hmerr::Result<(Calibration, Vec<(Q, f32)>)> {
	let rated = rated::of(entry, &[island::log_path()?, log::path()?])?;
	let point = predict::rated(db, &rated)?;

	Ok((Calibration::fit(&point), point))
}

fn nothing(best: Option<&(f32, Scored)>) {
	match best {
		Some((expected, scored)) => println!(
			"{Y}no candidate reaches q1{D}{F}, best{D} {expected:.0} {B}{mbid}{D}",
			mbid = scored.mbid
		),
		None => println!("{Y}no candidate at all{D}"),
	}
}

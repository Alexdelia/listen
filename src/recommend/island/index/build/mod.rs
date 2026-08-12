mod artist;
mod pool;
mod recording;
mod scan;
mod seed;
mod user_listen;

use std::{fs, path::Path};

use ansi::abbrev::{B, D, F, Y};
use chrono::{DateTime, Local, NaiveDateTime, SubsecRound, Utc};
use hmerr::ioe;

use crate::declaration::parse;

use super::{dump::Listen, open};

use scan::Scan;

const WORK: &str = "build";

pub(super) fn run(dir: &Path, dump: &Listen, declaration: &Path) -> hmerr::Result<()> {
	let declared = parse::parse(declaration)?;
	let work = dir.join(WORK);
	fs::create_dir_all(&work).map_err(|e| ioe!(work.to_string_lossy(), e))?;

	let started = Utc::now();
	announce(declared.len(), &started);

	let scan = Scan::of(&work, &dump.dir)?;

	seed::declare(&scan, &declared)?;
	let listen = seed::listen(&scan)?;
	let pool = pool::of(&scan, &listen, declared.len())?;
	let recording = recording::of(&scan, dir)?;
	artist::of(&scan, dir, &recording)?;
	let row = user_listen::of(&scan, dir, &pool, &recording)?;

	open::write_meta(
		dir,
		&open::Meta {
			built: Utc::now().date_naive().to_string(),
			dump: dump.name.clone(),
			seed: declared.len() as u64,
			user: scan.count(&pool)?,
			recording: scan.count(&recording)?,
			user_listen: row,
		},
	)?;

	println!(
		"{F}index built in {B}{Y}{elapsed}{D}",
		elapsed = elapsed(&started)
	);

	Ok(())
}

fn announce(declared: usize, started: &DateTime<Utc>) {
	println!(
		"\n{F}no index yet. building one from {B}{declared}{D}{F} declared recording.{D}\n\
		{F}this reads the whole listen dump and takes a while, tens of minutes on a warm disk.{D}\n\
		{F}started at {B}{Y}{at}{D}{F}, it only has to happen once per dump.{D}\n",
		at = wall_clock(started)
	);
}

fn wall_clock(at: &DateTime<Utc>) -> NaiveDateTime {
	at.with_timezone(&Local).naive_local().trunc_subsecs(0)
}

fn elapsed(started: &DateTime<Utc>) -> String {
	let second = (Utc::now() - *started).num_seconds().max(0);

	format!("{}m {}s", second / 60, second % 60)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn at(nanosecond: u32) -> DateTime<Utc> {
		DateTime::from_timestamp(1_786_000_000, nanosecond).unwrap_or_default()
	}

	#[test]
	fn a_wall_clock_reads_as_a_date_and_a_time_with_no_t_between_them() {
		let shown = wall_clock(&at(0)).to_string();

		assert!(!shown.contains('T'), "{shown}");
		assert_eq!(shown.len(), "2026-08-12 18:58:05".len(), "{shown}");
	}

	#[test]
	fn a_wall_clock_drops_the_subsecond_the_clock_came_with() {
		let shown = wall_clock(&at(123_456_789)).to_string();

		assert!(!shown.contains('.'), "{shown}");
		assert_eq!(shown, wall_clock(&at(0)).to_string());
	}
}

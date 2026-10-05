mod cohort;
mod converge;
mod log;
mod partition;
mod rank;
mod real;
mod report;
mod score;
mod seed;
mod select;

use std::{collections::HashSet, path::PathBuf};

use ansi::abbrev::{B, D, F, G, R};
use hmerr::{GenericError, ge};

use listen_index as index;

use crate::args::IslandArg;

use super::{
	attraction,
	feed::{self, Fed},
	local::Local,
	log::Log,
};

pub(super) use report::Report;

use partition::{Island, Request};
use seed::Library;

pub(super) fn log_path() -> hmerr::Result<PathBuf> {
	log::path()
}

pub(super) fn absent() {
	eprintln!("{F}no island index, {G}run --source island{D}{F} to build it{D}");
}

pub(super) fn feed(local: &Local, arg: &IslandArg, log: Log) -> hmerr::Result<Fed> {
	let index = &local.index;
	attraction::declare(&index.db)?;
	let library = seed::load(&local.entry, index)?;

	let request = request(arg);
	let tuning = score::Tuning {
		damp: arg.popularity_damp,
		allow_known_artist: arg.allow_known_artist,
		backing: if request.asked() {
			score::Backing::Reach
		} else {
			score::Backing::Head
		},
	};
	let converge::Converged { found, round } = if narrows(arg, &request) {
		converge::narrowed(converge::raise(
			index,
			&library,
			narrowed(&library, arg, &request)?,
			tuning,
		)?)
	} else {
		converge::of(
			index,
			&library,
			&partition::terrain(&library),
			arg.granularity,
			tuning,
		)?
	};

	let report = report::of(
		&report::Run {
			meta: &index.meta,
			library: &library,
			tuning,
			granularity: arg.granularity,
			mode: mode(arg, &request),
		},
		&found,
		round,
	);

	Ok(Fed {
		feed: Box::new(select::stream(
			found
				.island
				.into_iter()
				.map(|island| select::Island {
					name: island.name,
					member: island.member.len(),
				})
				.collect(),
			found.candidate,
			arg.ask,
			tuning,
			arg.granularity,
			log,
		)),
		report: feed::Report::Island(report),
	})
}

const fn mode(arg: &IslandArg, request: &Request) -> report::Mode {
	if request.asked() {
		return report::Mode::Requested;
	}

	if arg.island.is_some() {
		return report::Mode::Pinned;
	}

	report::Mode::Detected
}

const fn narrows(arg: &IslandArg, request: &Request) -> bool {
	request.asked() || arg.island.is_some()
}

fn narrowed(library: &Library, arg: &IslandArg, request: &Request) -> hmerr::Result<Vec<Island>> {
	let island = if request.asked() {
		partition::requested(library, request)?
	} else {
		partition::of(
			&partition::terrain(library),
			arg.granularity,
			&HashSet::new(),
		)
	};

	pin(island, arg.island.as_deref())
}

fn request(arg: &IslandArg) -> partition::Request {
	partition::Request {
		recording: arg.seed.clone(),
		genre: arg.genre.clone(),
	}
}

fn pin(island: Vec<Island>, name: Option<&str>) -> hmerr::Result<Vec<Island>> {
	let Some(name) = name else {
		return Ok(island);
	};

	let wanted = name.to_lowercase();
	let matching: Vec<Island> = island
		.into_iter()
		.filter(|island| island.name.contains(&wanted))
		.collect();

	if matching.is_empty() {
		return Err(unknown(name).into());
	}

	Ok(matching)
}

fn unknown(name: &str) -> GenericError {
	ge!(
		format!("{R}no island named {B}{name}{D}"),
		h: "islands are detected fresh every run, run without --island to see this run's names"
	)
}

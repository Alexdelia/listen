mod cohort;
mod index;
mod log;
mod partition;
mod real;
mod score;
mod seed;
mod select;

use std::path::Path;

use ansi::abbrev::{B, D, F, R, Y};
use hmerr::{GenericError, ge};

use crate::args::IslandArg;

pub(super) fn feed(path: &Path, arg: &IslandArg) -> hmerr::Result<Box<dyn super::feed::Feed>> {
	let index = index::open()?;
	let library = seed::load(path, &index)?;

	report(&index.meta, &library);

	let alpha = arg.alpha.unwrap_or(score::ALPHA);
	let resolution = arg.resolution.map_or(partition::RESOLUTION, f64::from);

	let island = partition::of(&library, resolution, &request(arg))?;
	let island = pin(island, arg.island.as_deref())?;

	let cohort: Vec<Vec<cohort::Member>> = island
		.iter()
		.map(|island| cohort::of(&library, island, cohort::SIZE))
		.collect();

	describe(&island, &cohort, &library);

	let shown = log::shown(&log::path()?)?;
	let candidate = score::of(&index, &cohort, &shown, alpha)?;

	Ok(Box::new(select::stream(
		island
			.into_iter()
			.map(|island| select::Island {
				name: island.name,
				member: island.member.len(),
			})
			.collect(),
		candidate,
		arg.ask,
		alpha,
		resolution,
		log::path()?,
	)))
}

fn request(arg: &IslandArg) -> partition::Request {
	partition::Request {
		recording: arg.seed.clone(),
		genre: arg.genre.clone(),
	}
}

fn pin(
	island: Vec<partition::Island>,
	name: Option<&str>,
) -> hmerr::Result<Vec<partition::Island>> {
	let Some(name) = name else {
		return Ok(island);
	};

	let wanted = name.to_lowercase();
	let matching: Vec<partition::Island> = island
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
		h: "islands are detected fresh every run, so run without --island to see this run's names"
	)
}

fn report(meta: &index::Meta, library: &seed::Library) {
	println!(
		"{F}index {Y}{built}{D}{F}, {B}{recording}{D}{F} recording and {B}{listen}{D}{F} listen over {B}{user}{D}{F} user{D}",
		built = meta.built,
		recording = meta.recording,
		listen = meta.user_listen,
		user = meta.user,
	);

	let unsupported = library.unsupported();
	if unsupported > 0 {
		println!(
			"{F}{unsupported} of {declared} declared recording have no listener in the index{D}",
			declared = library.declared.len(),
		);
	}
}

fn describe(island: &[partition::Island], cohort: &[Vec<cohort::Member>], library: &seed::Library) {
	for (island, cohort) in island.iter().zip(cohort) {
		println!(
			"{B}{name}{D} {F}q{q:.2}, {member} seed, {size} user{D}",
			name = island.name,
			q = island.q(&library.seed),
			member = island.member.len(),
			size = cohort.len(),
		);
	}
}

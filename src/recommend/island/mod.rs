mod attraction;
mod cohort;
mod index;
mod log;
mod partition;
mod rank;
mod real;
mod score;
mod seed;
mod select;

use std::path::Path;

use ansi::abbrev::{B, CYA, D, F, G, M, R, Y};
use hmerr::{GenericError, ge};

use crate::{
	args::IslandArg,
	format::{self, genre_list, human_readable_number},
};

pub(super) fn ready() -> bool {
	index::ready()
}

pub(super) fn absent() {
	println!("{F}no island index, {G}run --source island{D}{F} to build it{D}");
}

pub(super) fn feed(path: &Path, arg: &IslandArg) -> hmerr::Result<Box<dyn super::feed::Feed>> {
	let index = index::ensure(path)?;
	let library = seed::load(path, &index)?;

	report(&index.meta, &library);

	let damp = arg.popularity_damp.unwrap_or(score::POPULARITY_DAMP);
	let granularity = arg.granularity.map_or(partition::GRANULARITY, f64::from);

	let island = partition::of(&library, granularity, &request(arg))?;
	let island = pin(island, arg.island.as_deref())?;

	let cohort: Vec<Vec<cohort::Member>> = island
		.iter()
		.map(|island| cohort::of(&library, island, cohort::SIZE))
		.collect();

	let (island, cohort) = rank::by_promise(island, cohort, &library);

	describe(&island, &cohort, &library);

	let candidate = score::of(&index, &cohort, damp)?;

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
		damp,
		granularity,
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
		h: "islands are detected fresh every run, run without --island to see this run's names"
	)
}

fn report(meta: &index::Meta, library: &seed::Library) {
	println!(
		"index {CYA}{built}{D}: {G}{recording} {G}{F}recording{D} {M}{listen} {F}listen{D} {CYA}{user} {F}user{D}",
		built = meta.built,
		recording = human_readable_number::text(meta.recording),
		listen = human_readable_number::text(meta.user_listen),
		user = human_readable_number::text(meta.user),
	);

	let unsupported = library.unsupported();
	if unsupported > 0 {
		println!(
			"{unsupported}{F}/{D}{declared} {F}declared recording have no listener in the index{D}",
			declared = library.declared.len(),
		);
	}

	println!();
}

fn describe(island: &[partition::Island], cohort: &[Vec<cohort::Member>], library: &seed::Library) {
	let width = island
		.iter()
		.map(|island| genre_list::width(&island.name))
		.max()
		.unwrap_or_default();

	for (island, cohort) in island.iter().zip(cohort) {
		let q = island.q(&library.seed);
		println!(
			"{name}{pad} {Y}{promise:.2} {F}promise  {q_color}{q:.2}{D} {CYA}{size:>4} {F}user{D} {G}{member:>4} {F}seed{D}",
			name = genre_list::text(&island.name),
			pad = genre_list::pad(&island.name, width),
			promise = rank::promise(island, cohort.len(), library),
			q_color = format::q_f32_color(q),
			member = island.member.len(),
			size = cohort.len(),
		);
	}
}

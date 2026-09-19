mod attraction;
mod cohort;
mod converge;
mod log;
mod partition;
mod rank;
mod real;
mod score;
mod seed;
mod select;

use std::{collections::HashSet, path::Path};

use ansi::abbrev::{B, CYA, D, F, G, M, R, Y};
use hmerr::{GenericError, ge};

use listen_index as index;

use crate::{
	args::IslandArg,
	ask,
	declaration::{Entry, parse},
	format::{self, genre_list, human_readable_number},
};

use partition::{Island, Request};
use seed::Library;

pub(super) fn ready() -> bool {
	index::ready()
}

pub(super) fn absent() {
	println!("{F}no island index, {G}run --source island{D}{F} to build it{D}");
}

pub(super) fn feed(path: &Path, arg: &IslandArg) -> hmerr::Result<Box<dyn super::feed::Feed>> {
	let entry = parse::parse(path)?;
	let index = index::ensure(&declared(&entry), &ask::Terminal)?;
	attraction::declare(&index.db)?;
	let library = seed::load(&entry, &index)?;

	report(&index.meta, &library);

	let request = request(arg);
	let found = if narrows(arg, &request) {
		converge::raise(
			&index,
			&library,
			narrowed(&library, arg, &request)?,
			arg.popularity_damp,
		)?
		.live()
	} else {
		converge::of(
			&index,
			&library,
			&partition::terrain(&library),
			arg.granularity,
			arg.popularity_damp,
		)?
	};

	describe(&found.island, &found.cohort, &library);

	Ok(Box::new(select::stream(
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
		arg.popularity_damp,
		arg.granularity,
		log::path()?,
	)))
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

fn declared(entry: &[Entry]) -> Vec<index::Seed> {
	entry
		.iter()
		.map(|entry| index::Seed {
			mbid: entry.s,
			q: entry.q,
		})
		.collect()
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

fn report(meta: &index::Meta, library: &Library) {
	println!(
		"index {CYA}{built}{D}: {G}{recording} {G}{F}recording{D} {M}{listen} {M}{F}listen{D} {CYA}{user} {F}user{D}",
		built = meta.built,
		recording = human_readable_number::text(meta.recording),
		listen = human_readable_number::text(meta.user_listen),
		user = human_readable_number::text(meta.user),
	);

	if meta.absorbed > 0 {
		println!(
			"covered to {CYA}{covered}{D} {F}after{D} {CYA}{absorbed}{D} {F}incremental{D}",
			covered = day(meta.covered()),
			absorbed = meta.absorbed
		);
	}

	for gap in &meta.gap {
		println!(
			"{Y}gap{D} {F}from{D} {CYA}{from}{D} {F}to{D} {CYA}{to}{D}",
			from = day(&gap.from),
			to = day(&gap.to)
		);
	}

	let unsupported = library.unsupported();
	if unsupported > 0 {
		println!(
			"{unsupported}{F}/{D}{declared} {F}declared recording have no listener in the index{D}",
			declared = library.declared.len(),
		);
	}

	println!();
}

fn day(timestamp: &str) -> &str {
	timestamp.split(' ').next().unwrap_or(timestamp)
}

fn describe(island: &[Island], cohort: &[Vec<cohort::Member>], library: &Library) {
	if island.is_empty() {
		println!("{Y}no island has a candidate{D}");

		return;
	}

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

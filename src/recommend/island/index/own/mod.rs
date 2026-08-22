mod board;
#[cfg(test)]
mod fixture;
mod fold;
mod reach;
mod scan;

use ansi::abbrev::{B, D, F, Y};

use super::{
	dump::{self, Pending},
	open, progress,
};

pub(crate) use open::Gap;
pub(crate) use scan::Play;

const AT_ONCE: u64 = 2;

pub(crate) struct Own {
	pub dump: String,
	pub covered: i64,
	pub play: Vec<Play>,
}

pub(crate) struct Fresh {
	pub reached: String,
	pub covered: i64,
	pub play: Vec<Play>,
	pub gap: Vec<Gap>,
}

pub(crate) fn unpacked() -> hmerr::Result<Option<String>> {
	Ok(dump::unpacked()?.map(|listen| listen.name))
}

pub(crate) fn played() -> hmerr::Result<Option<Own>> {
	let dir = open::dir()?;

	let Some(own) = open::own(&dir) else {
		return Ok(None);
	};
	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	let scanned = scan::of(&open::session(&dir)?, &listen.dir, own)?;

	if scanned.play.is_empty() {
		return Ok(None);
	}

	Ok(Some(Own {
		dump: listen.name,
		covered: scanned.covered,
		play: scanned.play,
	}))
}

pub(crate) fn fresh(reached: &str) -> hmerr::Result<Option<Fresh>> {
	let dir = open::dir()?;

	let Some(own) = open::own(&dir) else {
		return Ok(None);
	};

	let pending = dump::pending(reached)?;
	let pending: Vec<&Pending> = pending.iter().collect();

	if pending.is_empty() || !offered(&pending)? {
		return Ok(None);
	}

	let root = dump::root()?;
	dump::room(&root, &pending, AT_ONCE)?;

	fold::run(&open::session(&dir)?, &root, &pending, own, reached).map(Some)
}

fn offered(pending: &[&Pending]) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}{B}{count}{D}{F} incremental dump published since those counts were read, \
		{B}{Y}{size}{D}{F}, each read once then deleted{D}",
		count = pending.len(),
		size = progress::bytes(dump::weight(pending))
	));

	progress::ask("download", true)
}

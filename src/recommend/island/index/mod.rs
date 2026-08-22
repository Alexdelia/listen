mod absorb;
mod board;
mod build;
mod dump;
mod keep;
mod open;
mod partial;
mod progress;
mod query;
mod shard;
mod user_stat;
mod work;

use std::path::Path;

use ansi::abbrev::{B, D, F, Y};

use dump::{Listen, Pending};

pub(super) use open::{Index, Meta};

pub(super) fn ready() -> bool {
	open::dir().is_ok_and(|dir| open::built(&dir))
}

pub(super) fn ensure(declaration: &Path) -> hmerr::Result<Index> {
	let dir = open::dir()?;

	if let Some(listen) = to_build_from(&dir)? {
		rebuilt(&dir, &listen, declaration)?;
	} else {
		user_stat::derive(&dir)?;

		if let Some(listen) = repaired(&dir)? {
			rebuilt(&dir, &listen, declaration)?;
		}
	}

	absorbed(&dir)?;

	dump::artist_link(&dir.join(open::ARTIST_LINK))?;

	open::open(&dir)
}

fn to_build_from(dir: &Path) -> hmerr::Result<Option<Listen>> {
	if !open::scanned(dir) {
		return dump::listen().map(Some);
	}

	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	Ok(asked(&listen)?.then_some(listen))
}

fn asked(listen: &Listen) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}listen dump {B}{name}{D}{F} unpacked next to a built index, \
		rebuilding replaces it and may be long{D}",
		name = listen.name
	));

	progress::ask("rebuild index", true)
}

fn rebuilt(dir: &Path, listen: &Listen, declaration: &Path) -> hmerr::Result<()> {
	build::run(dir, listen, declaration)?;

	dump::discard(listen)
}

fn repaired(dir: &Path) -> hmerr::Result<Option<Listen>> {
	let meta = open::meta(dir)?;

	if meta.gap.is_empty() {
		return Ok(None);
	}

	let Some(dump) = listed(dump::repairable(meta.covered())).flatten() else {
		return Ok(None);
	};

	dump::repair(&dump)
}

fn absorbed(dir: &Path) -> hmerr::Result<()> {
	let meta = open::meta(dir)?;

	let Some(pending) = listed(dump::pending(meta.covered())) else {
		return Ok(());
	};

	if pending.is_empty() || !offered(&pending)? {
		return Ok(());
	}

	absorb::run(dir, &meta, &pending)
}

fn offered(pending: &[Pending]) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}absorbing {B}{count}{D}{F} incremental dump, {B}{Y}{size}{D}{F}, \
		each read once then deleted{D}",
		count = pending.len(),
		size = progress::bytes(dump::weight(pending))
	));

	progress::ask("download", true)
}

fn listed<T>(published: hmerr::Result<T>) -> Option<T> {
	match published {
		Ok(published) => Some(published),
		Err(e) => {
			progress::say(format!(
				"{F}cannot read what is published, keeping the index as it stands{D}\n{e}"
			));

			None
		}
	}
}

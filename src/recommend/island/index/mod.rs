mod absorb;
mod board;
mod build;
mod dump;
mod keep;
mod open;
mod parallel;
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

	let built = open::meta(dir)?.dump;

	if !newer(&listen, &built) {
		return Ok(None);
	}

	Ok(asked(&listen, &built)?.then_some(listen))
}

fn newer(listen: &Listen, built: &str) -> bool {
	dump::reach(&listen.name)
		.ok()
		.zip(dump::reach(built).ok())
		.is_none_or(|(unpacked, built)| unpacked > built)
}

fn asked(listen: &Listen, built: &str) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}listen dump {B}{name}{D}{F} unpacked next to an index built from {B}{built}{D}{F}, \
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

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::*;

	const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";

	fn listen(name: &str) -> Listen {
		Listen {
			dir: PathBuf::new(),
			name: name.to_string(),
		}
	}

	#[test]
	fn the_dump_an_index_was_built_from_is_never_one_to_rebuild_from() {
		assert!(!newer(&listen(BUILT), BUILT));
	}

	#[test]
	fn a_dump_reaching_past_the_index_baseline_is_one_to_rebuild_from() {
		assert!(newer(&listen("2026-08-16 00:00:03.000000+00:00"), BUILT));
	}

	#[test]
	fn a_dump_older_than_the_index_baseline_is_left_where_it_lies() {
		assert!(!newer(&listen("2026-06-01 00:00:03.000000+00:00"), BUILT));
	}

	#[test]
	fn a_dump_whose_name_says_nothing_is_still_offered() {
		assert!(newer(&listen("listen"), BUILT));
	}
}

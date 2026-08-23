#[cfg(test)]
mod fixture;
mod fold;
mod reach;
mod scan;
mod stage;

use std::path::Path;

use ansi::abbrev::{B, D, F, Y};

use super::{
	decide::Decide,
	dump::{self, Listen, Pending},
	index, listener, progress,
};

pub use index::Gap;
pub use scan::Play;

const AT_ONCE: u64 = 2;

pub struct Own {
	pub dump: String,
	pub covered: i64,
	pub play: Vec<Play>,
}

pub struct Fold {
	pub reached: String,
	pub covered: i64,
	pub play: Vec<Play>,
	pub gap: Vec<Gap>,
}

pub fn unpacked() -> hmerr::Result<Option<String>> {
	Ok(dumped()?.map(|listen| listen.name))
}

pub fn played(username: &str) -> hmerr::Result<Option<Own>> {
	let dir = index::dir()?;

	let Some(own) = listened_as(&dir, username)? else {
		return Ok(None);
	};
	let Some(listen) = dumped()? else {
		return Ok(None);
	};

	let scanned = scan::of(&index::session::of(&dir)?, &listen.dir, own)?;

	if scanned.play.is_empty() {
		return Ok(None);
	}

	Ok(Some(Own {
		dump: listen.name,
		covered: scanned.covered,
		play: scanned.play,
	}))
}

pub fn fresh(
	username: &str,
	reached: &str,
	decide: &dyn Decide,
	keep: &mut impl FnMut(Fold) -> hmerr::Result<()>,
) -> hmerr::Result<()> {
	let dir = index::dir()?;

	let Some(own) = listened_as(&dir, username)? else {
		return Ok(());
	};

	if !stamped(reached) {
		unreadable(reached);
		return Ok(());
	}

	let Some(pending) = published(dump::pending(reached)) else {
		return Ok(());
	};
	let pending: Vec<&Pending> = pending.iter().collect();

	if pending.is_empty() || !offered(&pending, decide)? {
		return Ok(());
	}

	let root = dump::root()?;
	dump::room(&root, &pending, AT_ONCE)?;

	fold::run(
		&index::session::of(&dir)?,
		&root,
		&pending,
		own,
		reached,
		keep,
	)
}

fn listened_as(dir: &Path, username: &str) -> hmerr::Result<Option<u32>> {
	if let Some(named) = listener::of(username)? {
		return Ok(Some(named));
	}

	Ok(index::meta::own(dir))
}

fn dumped() -> hmerr::Result<Option<Listen>> {
	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	if !stamped(&listen.name) {
		unstamped(&listen.name);
		return Ok(None);
	}

	Ok(Some(listen))
}

#[must_use]
pub fn stamped(name: &str) -> bool {
	dump::reach(name).is_ok()
}

fn unreadable(reached: &str) {
	progress::say(format!(
		"{Y}the counts stop at {B}{reached}{D}{Y}, which is no timestamp to hold a dump against, \
		nothing is folded onto them, {B}--refresh{D}{Y} reads the dump up again to clear it{D}"
	));
}

fn unstamped(name: &str) {
	progress::say(format!(
		"{Y}the unpacked dump {B}{name}{D}{Y} carries no readable timestamp, \
		its listens stay out of the counts until it is fetched again{D}"
	));
}

fn published(pending: hmerr::Result<Vec<Pending>>) -> Option<Vec<Pending>> {
	match pending {
		Ok(pending) => Some(pending),
		Err(e) => {
			progress::say(format!(
				"{F}cannot read what is published, keeping the counts as they stand{D}\n{e}"
			));

			None
		}
	}
}

fn offered(pending: &[&Pending], decide: &dyn Decide) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}{B}{count}{D}{F} incremental dump published since those counts were read, \
		{B}{Y}{size}{D}{F}, each read once then deleted{D}",
		count = pending.len(),
		size = progress::bytes(dump::weight(pending))
	));

	progress::confirm(decide, "download", true)
}

#[cfg(test)]
mod tests {
	use hmerr::ge;

	use super::*;

	#[test]
	fn a_dump_directory_without_its_timestamp_is_no_dump_to_count_off() {
		assert!(stamped("2026-07-12 00:00:04.001868+00:00"));
		assert!(stamped("2026-07-12 00:00:04"));
		assert!(!stamped("listen"));
		assert!(!stamped("LATEST"));
		assert!(!stamped(""));
	}

	#[test]
	fn nothing_published_can_be_read_leaves_the_counts_where_they_are() {
		assert!(published(Err(ge!("rsync is not here".to_string()).into())).is_none());
	}

	#[test]
	fn what_is_published_is_read_as_it_comes() {
		assert!(published(Ok(Vec::new())).is_some_and(|pending| pending.is_empty()));
	}
}

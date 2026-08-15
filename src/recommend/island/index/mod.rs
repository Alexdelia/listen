mod build;
mod dump;
mod keep;
mod open;
mod partial;
mod progress;
mod user_stat;

use std::path::Path;

use ansi::abbrev::{B, D, F};
use hmerr::ioe;

use dump::Listen;

pub(super) use open::{Index, Meta};

pub(super) fn ready() -> bool {
	open::dir().is_ok_and(|dir| open::built(&dir))
}

pub(super) fn ensure(declaration: &Path) -> hmerr::Result<Index> {
	let dir = open::dir()?;

	user_stat::derive(&dir)?;

	if let Some(listen) = to_build_from(&dir)? {
		build::run(&dir, &listen, declaration)?;
		dump::discard(&listen)?;
	}

	dump::artist_link(&dir.join(open::ARTIST_LINK))?;

	open::open(&dir)
}

fn to_build_from(dir: &Path) -> hmerr::Result<Option<Listen>> {
	if !open::indexed(dir) {
		return dump::listen().map(Some);
	}

	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	Ok(asked(&listen)?.then_some(listen))
}

fn asked(listen: &Listen) -> hmerr::Result<bool> {
	println!(
		"\n{F}the listen dump {B}{name}{D}{F} is unpacked in the cache, next to an index that is \
		already built.{D}\n\
		{F}building again replaces that index and releases the dump, which takes a while.{D}",
		name = listen.name
	);

	ux::ask_yn("rebuild the index from it", true).map_err(|e| ioe!("stdin", e).into())
}

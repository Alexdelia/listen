mod age;
mod analyze;
mod cache;
mod dump;
mod fetch;
mod gap;
mod interactive;
mod meta;
mod render;
mod song;

use std::path::Path;

use ansi::abbrev::{B, CYA, D, G};

use crate::declaration::parse;

use fetch::ListenCount;
use gap::Covered;

struct Listened {
	count: ListenCount,
	covered: Covered,
}

pub(crate) fn run(
	path: &Path,
	username: Option<&str>,
	refresh: bool,
	interactive: bool,
	api: bool,
) -> hmerr::Result<()> {
	let username = crate::cache::username::resolve(username)?;

	let list = parse::parse(path)?;
	let listened = listened(&username, refresh, api)?;
	let age = age::days_since_added(path)?;
	let meta = meta::declared(&list);

	let analysis = analyze::analyze(&list, &listened.count, &age, &meta, &listened.covered);

	if interactive {
		return interactive::run(&analysis, path, &username);
	}

	render::render(&analysis, &username);

	Ok(())
}

fn listened(username: &str, refresh: bool, api: bool) -> hmerr::Result<Listened> {
	if !api && let Some(held) = dump::listen(username, refresh)? {
		return Ok(Listened {
			covered: gap::covered(held.ago()?, &held.gap)?,
			count: held.counted(),
		});
	}

	Ok(Listened {
		count: listen(username, refresh)?,
		covered: gap::covered(0, &[])?,
	})
}

fn listen(username: &str, refresh: bool) -> hmerr::Result<ListenCount> {
	if !refresh && let Some(cached) = cache::listen::read(username)? {
		println!(
			"{B}{CYA}cached{D} listen stat for {B}{username}{D} ({B}--refresh{D} to update)\n"
		);
		return Ok(cached);
	}

	let listen = fetch::listen_count(username)?;
	cache::listen::write(username, &listen)?;
	println!(
		"{B}{G}fetched{D} {count} recording for {B}{username}{D}\n",
		count = listen.len()
	);

	Ok(listen)
}

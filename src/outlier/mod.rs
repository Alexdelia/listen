mod age;
mod analyze;
mod cache;
mod fetch;
mod interactive;
mod meta;
mod render;
mod song;

use std::path::Path;

use ansi::abbrev::{B, CYA, D, G};

use crate::declaration::parse;

use fetch::ListenCount;

pub fn run(
	path: &Path,
	username: Option<&str>,
	refresh: bool,
	interactive: bool,
) -> hmerr::Result<()> {
	let username = crate::cache::username::resolve(username)?;

	let list = parse::parse(path)?;
	let listen = listen(&username, refresh)?;
	let age = age::days_since_added(path)?;
	let meta = meta::declared(&list);

	let analysis = analyze::analyze(&list, &listen, &age, &meta);

	if interactive {
		return interactive::run(&analysis, path);
	}

	render::render(&analysis);

	Ok(())
}

fn listen(username: &str, refresh: bool) -> hmerr::Result<ListenCount> {
	if !refresh && let Some(cached) = cache::listen::read(username)? {
		println!(
			"{B}{CYA}cached{D} listen stats for {B}{username}{D} ({B}--refresh{D} to update)\n"
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

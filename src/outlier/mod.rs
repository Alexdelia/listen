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

use console::style;

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
) -> miette::Result<()> {
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

fn listened(username: &str, refresh: bool, api: bool) -> miette::Result<Listened> {
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

fn listen(username: &str, refresh: bool) -> miette::Result<ListenCount> {
	if !refresh && let Some(cached) = cache::listen::read(username)? {
		println!(
			"{} listen stat for {} ({} to update)\n",
			style("cached").bold().cyan(),
			style(&username).bold(),
			style("--refresh").bold()
		);
		return Ok(cached);
	}

	let listen = fetch::listen_count(username)?;
	cache::listen::write(username, &listen)?;
	println!(
		"{} {count} recording for {}\n",
		style("fetched").bold().green(),
		style(&username).bold(),
		count = listen.len()
	);

	Ok(listen)
}

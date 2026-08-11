mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
mod island;
mod listen_count;
mod queue;
mod recommendation;
mod selection;
mod skip;
mod stream;
mod target;
mod weekly;

use std::path::Path;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use feed::Feed;
use skip::Skip;
use stream::Stream;
use target::Target;

pub async fn run(
	path: &Path,
	target: Option<&str>,
	unlistened: bool,
	source: RecommendSource,
	sort: RecommendSort,
	arg: &IslandArg,
) -> hmerr::Result<()> {
	let mut feed: Vec<Box<dyn Feed>> = Vec::new();

	if selection::island(source) {
		selection::ensure_island(sort, target, arg)?;
		feed.push(island::feed(path, arg)?);
	} else {
		feed = remote(target, source, sort, arg).await?;
	}

	let mut skip = Skip::load(path)?;
	let mut stream = Stream::new(feed, unlistened);

	while let Some((index, recommendation)) = stream.next(&mut skip)? {
		if consider::consider(path, index, &recommendation)
			.await?
			.is_break()
		{
			return Ok(());
		}
	}

	Ok(())
}

async fn remote(
	target: Option<&str>,
	source: RecommendSource,
	sort: RecommendSort,
	arg: &IslandArg,
) -> hmerr::Result<Vec<Box<dyn Feed>>> {
	let target = target::resolve(target)?;
	selection::ensure(source, sort, &target)?;
	selection::ensure_no_island_arg(source, arg)?;

	let mut feed: Vec<Box<dyn Feed>> = Vec::new();

	if let Target::Username(username) = &target {
		if let Some(weekly) = weekly::feed(username, source)? {
			feed.push(Box::new(weekly));
		}

		if selection::collaborative_filtering(source) {
			feed.push(Box::new(collaborative_filtering::feed(username.clone())));
		}
	}

	if let Target::Artist(mbid) = target {
		feed.push(Box::new(listen_count::feed(mbid, sort).await?));
	}

	Ok(feed)
}

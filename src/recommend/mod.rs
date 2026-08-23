mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
pub(crate) mod island;
mod listen_count;
mod queue;
mod recommendation;
mod selection;
mod skip;
mod stream;
mod target;
mod turn;
mod weekly;

use std::path::Path;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use feed::Feed;
use skip::Skip;
use stream::Stream;
use target::Target;

pub(crate) async fn run(
	path: &Path,
	target: Option<&str>,
	unlistened: bool,
	source: RecommendSource,
	sort: RecommendSort,
	arg: &IslandArg,
) -> hmerr::Result<()> {
	selection::ensure_arg(source, arg)?;

	let feed: Vec<Box<dyn Feed>> = if selection::island_only(source) {
		selection::ensure_island_target(sort, target)?;

		vec![island::feed(path, arg)?]
	} else {
		let target = target::resolve(target)?;
		selection::ensure(source, sort, &target)?;

		let mut feed = remote(&target, source, sort).await?;

		if selection::island(source) && matches!(target, Target::Username(_)) {
			if island::ready() {
				feed.push(island::feed(path, arg)?);
			} else {
				island::absent();
			}
		}

		feed
	};

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
	target: &Target,
	source: RecommendSource,
	sort: RecommendSort,
) -> hmerr::Result<Vec<Box<dyn Feed>>> {
	let mut feed: Vec<Box<dyn Feed>> = Vec::new();

	if let Target::Username(username) = target {
		if let Some(weekly) = weekly::feed(username, source)? {
			feed.push(Box::new(weekly));
		}

		if selection::collaborative_filtering(source) {
			feed.push(Box::new(collaborative_filtering::feed(username.clone())));
		}
	}

	if let Target::Artist(mbid) = target {
		feed.push(Box::new(listen_count::feed(*mbid, sort).await?));
	}

	Ok(feed)
}

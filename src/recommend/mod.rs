mod attraction;
mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
pub(crate) mod island;
mod known_artist;
mod listen_count;
mod local;
mod queue;
mod recommendation;
mod selection;
mod similar;
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
		selection::ensure_local_target(source, sort, target)?;

		vec![island::feed(&local::open(path)?, arg)?]
	} else if selection::similar_only(source) {
		selection::ensure_local_target(source, sort, target)?;

		let local = local::open(path)?;
		if arg.backtest {
			return similar::backtest(&local);
		}

		vec![similar::feed(&local, arg)?]
	} else {
		let target = target::resolve(target)?;
		selection::ensure(source, sort, &target)?;

		let mut feed = remote(&target, source, sort).await?;

		if matches!(target, Target::Username(_)) {
			feed.extend(local_feed(path, source, arg)?);
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

fn local_feed(
	path: &Path,
	source: RecommendSource,
	arg: &IslandArg,
) -> hmerr::Result<Vec<Box<dyn Feed>>> {
	let island = selection::island(source);
	let similar = selection::similar(source);

	if !island && !similar {
		return Ok(Vec::new());
	}

	if !local::ready() {
		island::absent();
		return Ok(Vec::new());
	}

	let local = local::open(path)?;
	let mut feed = Vec::new();

	if island {
		feed.push(island::feed(&local, arg)?);
	}

	if similar {
		feed.push(similar::feed(&local, arg)?);
	}

	Ok(feed)
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

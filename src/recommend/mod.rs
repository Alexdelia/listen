mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
mod listen_count;
mod queue;
mod recommendation;
mod selection;
mod skip;
mod stream;
mod target;
mod weekly;

use std::path::Path;

use crate::args::{RecommendSort, RecommendSource};

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
) -> hmerr::Result<()> {
	let target = target::resolve(target)?;
	selection::ensure(source, sort, &target)?;

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

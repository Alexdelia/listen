mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
mod recommendation;
mod selection;
mod skip;
mod stream;
mod weekly;

use std::path::Path;

use crate::{args::RecommendSource, cache};

use feed::Feed;
use skip::Skip;
use stream::Stream;

pub async fn run(
	path: &Path,
	username: Option<&str>,
	unlistened: bool,
	source: RecommendSource,
) -> hmerr::Result<()> {
	let username = cache::username::resolve(username)?;

	let weekly = weekly::feed(&username, source)?;
	let collaborative_filtering =
		selection::collaborative_filtering(source).then(|| collaborative_filtering::feed(username));

	let mut skip = Skip::load(path)?;
	let mut stream = Stream::new(
		weekly.map(|feed| Box::new(feed) as Box<dyn Feed>),
		collaborative_filtering.map(|feed| Box::new(feed) as Box<dyn Feed>),
		unlistened,
	);

	let mut index = 0;
	while let Some(recommendation) = stream.next(&mut skip)? {
		if consider::consider(path, index, &recommendation)
			.await?
			.is_break()
		{
			return Ok(());
		}

		index += 1;
	}

	Ok(())
}

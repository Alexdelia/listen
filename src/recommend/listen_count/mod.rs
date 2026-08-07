mod catalogue;
mod fetch;
mod payload;
mod rank;
mod render;

use crate::{args::RecommendSort, declaration::Source};

use super::queue::Queue;

use catalogue::catalogue;
use payload::popularity;
use rank::rank;

pub(super) async fn feed(mbid: Source, sort: RecommendSort) -> hmerr::Result<Queue> {
	let catalogue = catalogue(mbid).await?;

	let mut found = Vec::new();
	for batch in catalogue.recording.chunks(fetch::BATCH) {
		found.extend(popularity(&fetch::popularity(batch)?)?);
	}

	let ranked = rank(sort, &catalogue, found);

	println!("{}", render::header(sort, &catalogue, ranked.len()));

	Ok(Queue::new(ranked))
}

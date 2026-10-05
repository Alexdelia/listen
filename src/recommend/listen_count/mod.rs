mod catalogue;
mod fetch;
mod payload;
mod rank;
mod render;
mod report;

use crate::{args::RecommendSort, declaration::Source};

use super::{
	feed::{self, Fed},
	queue::Queue,
};

use catalogue::catalogue;
use payload::popularity;
use rank::rank;

pub(super) use report::Report;

pub(super) async fn feed(mbid: Source, sort: RecommendSort) -> hmerr::Result<Fed> {
	let catalogue = catalogue(mbid).await?;

	let mut found = Vec::new();
	for batch in catalogue.recording.chunks(fetch::BATCH) {
		found.extend(popularity(&fetch::popularity(batch)?)?);
	}

	let ranked = rank(sort, &catalogue, found);
	let report = Report {
		artist: catalogue.artist.clone(),
		sort,
		recording: catalogue.recording.len(),
		dated: catalogue.released.len(),
		ranked: ranked.len(),
	};

	Ok(Fed {
		feed: Box::new(Queue::new(ranked)),
		report: feed::Report::ListenCount(report),
	})
}

mod catalogue;
mod fetch;
mod payload;
mod rank;

use ansi::abbrev::{B, D, F};

use crate::declaration::Source;

use super::queue::Queue;

use catalogue::catalogue;
use payload::popularity;
use rank::rank;

pub(super) async fn feed(mbid: Source) -> hmerr::Result<Queue> {
	let catalogue = catalogue(mbid).await?;

	let mut found = Vec::new();
	for batch in catalogue.recording.chunks(fetch::BATCH) {
		found.extend(popularity(&fetch::popularity(batch)?)?);
	}

	let ranked = rank(found);

	println!(
		"{B}{artist}{D} {F}{listened} listened of {total} recording{D}",
		artist = catalogue.artist,
		listened = ranked.len(),
		total = catalogue.recording.len(),
	);

	Ok(Queue::new(ranked))
}

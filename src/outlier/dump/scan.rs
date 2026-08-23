use crate::recommend::island::index::own;

use super::{
	super::{cache, fetch::Listen},
	Held,
	held::Carried,
	say,
};

pub(super) fn scanned(username: &str, carried: Option<Carried>) -> hmerr::Result<Option<Held>> {
	say::reading();

	let Some(own) = own::played(username)? else {
		return Ok(None);
	};

	let carried = carried.unwrap_or_else(|| Carried::of(&own.dump));

	let held = Held {
		dump: own.dump,
		reached: carried.reached,
		gap: carried.gap,
		covered: carried.covered.max(own.covered),
		count: own
			.play
			.into_iter()
			.map(|play| {
				(
					play.mbid,
					Listen {
						count: play.plays,
						track: play.track,
						artist: play.artist,
					},
				)
			})
			.collect(),
		fold: Some(carried.fold),
	};

	cache::dump::write(username, &held)?;

	Ok(Some(held))
}

use std::path::Path;

use super::{super::open::USER_LISTEN, library, pool::Pool, scan::Scan};

pub(super) const NAME: &str = "listen";

pub(super) fn of(
	scan: &Scan,
	dir: &Path,
	library: &Path,
	pool: &Pool,
	recording: &Path,
) -> hmerr::Result<u64> {
	let into = dir.join(USER_LISTEN);

	scan.bucketed(&into, NAME, &|bucket| {
		format!(
			r"
select l.user_id, r.recording_id, l.plays
from {library} l
semi join {pool} u on u.user_id = l.user_id
join read_parquet('{recording}') r on r.mbid = l.mbid
",
			library = library::read_bucket(library, bucket),
			pool = pool.read(),
			recording = recording.display()
		)
	})?;

	scan.count(&into.join("*.parquet"))
}

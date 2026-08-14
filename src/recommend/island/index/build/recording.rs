use std::path::{Path, PathBuf};

use super::{super::open::RECORDING, scan::Scan};

const MIN_GLOBAL_PLAY: u64 = 5;

pub(super) fn of(scan: &Scan, dir: &Path) -> hmerr::Result<PathBuf> {
	let partial = scan.batched("recording", &|shard| {
		format!(
			r"
select l.recording_mbid::uuid as mbid, count(*) as global_plays
from read_parquet({shard}) l
where l.recording_mbid is not null
group by 1
"
		)
	})?;

	let into = dir.join(RECORDING);
	scan.copy(
		&into,
		&format!(
			r"
select
	(row_number() over (order by mbid) - 1)::uinteger as recording_id,
	mbid,
	sum(global_plays)::uinteger as global_plays
from read_parquet('{partial}/*.parquet')
group by mbid
having sum(global_plays) >= {MIN_GLOBAL_PLAY}
	or mbid in (select mbid from seed)
",
			partial = partial.display()
		),
	)?;

	Ok(into)
}

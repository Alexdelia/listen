use std::path::Path;

use super::{super::open::RECORDING_ARTIST, scan::Scan};

pub(super) const NAME: &str = "artist";

const CREDIT: &str = "credit";

pub(super) fn of(scan: &Scan, dir: &Path, recording: &Path) -> hmerr::Result<()> {
	let recording = recording.display().to_string();

	let partial = scan.batched(NAME, &|shard| {
		format!(
			r"
select distinct r.recording_id, unnest(l.artist_credit_mbids)::uuid as artist_mbid
from read_parquet({shard}) l
join read_parquet('{recording}') r on r.mbid::varchar = l.recording_mbid
where l.artist_credit_mbids is not null
"
		)
	})?;

	scan.step(
		CREDIT,
		&dir.join(RECORDING_ARTIST),
		&format!(
			r"
select distinct recording_id, artist_mbid
from read_parquet('{partial}/*.parquet')
",
			partial = partial.display()
		),
	)
}

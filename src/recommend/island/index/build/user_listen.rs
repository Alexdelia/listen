use std::{fs, path::Path};

use hmerr::ioe;

use super::{
	super::{open::USER_LISTEN, progress},
	scan::Scan,
};

const BUCKET: u32 = 8;
const PLAY_CEILING: u32 = 65535;

pub(super) fn of(scan: &Scan, dir: &Path, pool: &Path, recording: &Path) -> hmerr::Result<u64> {
	let pool = pool.display().to_string();
	let recording = recording.display().to_string();

	let partial = scan.batched("listen", &|shard| {
		format!(
			r"
select
	l.user_id::uinteger as user_id,
	r.recording_id,
	least(count(*), {PLAY_CEILING})::usmallint as plays
from read_parquet({shard}) l
semi join read_parquet('{pool}') u on u.user_id = l.user_id
join read_parquet('{recording}') r on r.mbid::varchar = l.recording_mbid
group by 1, 2
"
		)
	})?;

	let into = dir.join(USER_LISTEN);
	fs::create_dir_all(&into).map_err(|e| ioe!(into.to_string_lossy(), e))?;

	let bar = progress::step_bar(u64::from(BUCKET), "compact")?;
	for bucket in 0..BUCKET {
		let shard = into.join(format!("{bucket}.parquet"));

		if !scan.done(&shard) {
			scan.copy(
				&shard,
				&format!(
					r"
select user_id, recording_id, least(sum(plays), {PLAY_CEILING})::usmallint as plays
from read_parquet('{partial}/*.parquet')
where user_id % {BUCKET} = {bucket}
group by 1, 2
",
					partial = partial.display()
				),
			)?;
		}

		bar.inc(1);
	}
	bar.finish();

	scan.count(&into.join("*.parquet"))
}

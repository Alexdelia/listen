use std::path::PathBuf;

use crate::declaration::Entry;

use super::scan::Scan;

pub(super) const NAME: &str = "seed";
const LISTEN: &str = "seed_listen.parquet";

pub(super) fn declare(scan: &Scan, declared: &[Entry]) -> hmerr::Result<()> {
	scan.db.execute_batch(&format!(
		r"create or replace table {NAME} (mbid uuid, q utinyint);"
	))?;

	let mut appender = scan.db.appender(NAME)?;
	for entry in declared {
		appender.append_row(duckdb::params![entry.s.to_string(), entry.q])?;
	}
	appender.flush()?;

	Ok(())
}

pub(super) fn listen(scan: &Scan) -> hmerr::Result<PathBuf> {
	let partial = scan.batched(NAME, &|shard| {
		format!(
			r"
select l.user_id, l.recording_mbid::uuid as mbid, count(*) as plays
from read_parquet({shard}) l
where l.recording_mbid in (select mbid::varchar from {NAME})
group by 1, 2
"
		)
	})?;

	let into = scan.work.join(LISTEN);
	scan.copy(
		&into,
		&format!(
			r"
select user_id, mbid, sum(plays) as plays
from read_parquet('{partial}/*.parquet')
group by 1, 2
",
			partial = partial.display()
		),
	)?;

	Ok(into)
}

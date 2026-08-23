use crate::Seed;

use super::scan::Scan;

pub(super) const NAME: &str = "seed";

pub(super) fn declare(scan: &Scan, declared: &[Seed]) -> hmerr::Result<()> {
	let db = scan.take();

	db.execute_batch(&format!(
		r"create or replace table {NAME} (mbid uuid, q utinyint);"
	))?;

	let mut appender = db.appender(NAME)?;
	for entry in declared {
		appender.append_row(duckdb::params![entry.mbid.to_string(), entry.q])?;
	}
	appender.flush()?;

	Ok(())
}

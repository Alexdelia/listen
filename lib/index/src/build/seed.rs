use miette::IntoDiagnostic;

use crate::Seed;

use super::scan::Scan;

pub(super) const NAME: &str = "seed";

pub(super) fn declare(scan: &Scan, declared: &[Seed]) -> miette::Result<()> {
	let db = scan.take();

	db.execute_batch(&format!(
		r"create or replace table {NAME} (mbid uuid, q utinyint);"
	))
	.into_diagnostic()?;

	let mut appender = db.appender(NAME).into_diagnostic()?;
	for entry in declared {
		appender
			.append_row(duckdb::params![entry.mbid.to_string(), entry.q])
			.into_diagnostic()?;
	}
	appender.flush().into_diagnostic()?;

	Ok(())
}

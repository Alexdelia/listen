use std::{collections::HashSet, path::Path};

use crate::declaration::{Entry, Source, parse};

pub(super) fn sources(path: &Path) -> hmerr::Result<HashSet<Source>> {
	Ok(parse::parse(path)?
		.into_iter()
		.map(|entry| entry.s)
		.collect())
}

pub(super) fn seed(entry: &[Entry]) -> Vec<listen_index::Seed> {
	entry
		.iter()
		.map(|entry| listen_index::Seed {
			mbid: entry.s,
			q: entry.q,
		})
		.collect()
}

pub(super) fn table(db: &duckdb::Connection, entry: &[Entry]) -> hmerr::Result<()> {
	db.execute_batch("create or replace temp table declared (mbid varchar, q utinyint);")?;

	let mut appender = db.appender("declared")?;
	for entry in entry {
		appender.append_row(duckdb::params![entry.s.to_string(), entry.q])?;
	}
	appender.flush()?;

	Ok(())
}

use std::path::Path;

use ansi::abbrev::{D, F};

use super::{
	super::attraction,
	open::{self, USER_LISTEN, USER_STAT},
	partial,
};

pub(super) fn derive(dir: &Path) -> hmerr::Result<()> {
	let into = dir.join(USER_STAT);

	if into.exists() || !open::bucketed(&dir.join(USER_LISTEN)) {
		return Ok(());
	}

	println!(
		"{F}index predates listener stat, derived from its listen, \
		rebuild from a dump to fix{D}"
	);

	let db = open::session(dir)?;

	partial::write(&into, |partial| {
		db.execute_batch(&format!(
			"copy ({stat}) to '{partial}' (format parquet, compression zstd);",
			stat = attraction::stat(&format!(
				"read_parquet('{dir}/{USER_LISTEN}/*.parquet')",
				dir = dir.display()
			)),
			partial = partial.display()
		))?;

		Ok(())
	})
}

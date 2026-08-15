use std::path::Path;

use ansi::abbrev::{D, F};

use super::{
	super::attraction,
	open::{self, USER_LISTEN, USER_STAT},
	partial, progress,
};

pub(super) fn derive(dir: &Path) -> hmerr::Result<()> {
	if !open::predates_stat(dir) {
		return Ok(());
	}

	let into = dir.join(USER_STAT);

	progress::say(format!(
		"{F}index predates listener stat, derived from its listen, \
		rebuild from a dump to fix{D}"
	));

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

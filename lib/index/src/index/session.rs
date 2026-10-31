use std::{fs, path::Path};

use miette::{IntoDiagnostic, WrapErr};

const MEMORY_LIMIT: &str = "4GB";
const SPILL: &str = "spill";

pub(crate) fn of(dir: &Path) -> miette::Result<duckdb::Connection> {
	let spill = dir.join(SPILL);
	fs::create_dir_all(&spill)
		.into_diagnostic()
		.wrap_err_with(|| format!("cannot create {}", spill.display()))?;

	let db = duckdb::Connection::open_in_memory().into_diagnostic()?;

	db.execute_batch(&format!(
		r"
set memory_limit='{MEMORY_LIMIT}';
set temp_directory='{spill}';
set preserve_insertion_order=false;
",
		spill = spill.display()
	))
	.into_diagnostic()?;

	Ok(db)
}

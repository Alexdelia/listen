use std::{fs, path::Path};

use hmerr::ioe;

const MEMORY_LIMIT: &str = "4GB";
const SPILL: &str = "spill";

pub(crate) fn of(dir: &Path) -> hmerr::Result<duckdb::Connection> {
	let spill = dir.join(SPILL);
	fs::create_dir_all(&spill).map_err(|e| ioe!(spill.to_string_lossy(), e))?;

	let db = duckdb::Connection::open_in_memory()?;

	db.execute_batch(&format!(
		r"
set memory_limit='{MEMORY_LIMIT}';
set temp_directory='{spill}';
set preserve_insertion_order=false;
",
		spill = spill.display()
	))?;

	Ok(db)
}

use std::path::Path;

use super::partial;

pub(super) fn copy(db: &duckdb::Connection, into: &Path, select: &str) -> hmerr::Result<()> {
	partial::write(into, |partial| {
		db.execute_batch(&format!(
			r"
copy ({select}) to '{partial}' (format parquet, compression zstd);
",
			partial = partial.display()
		))?;

		Ok(())
	})
}

pub(super) fn done(db: &duckdb::Connection, of: &Path) -> bool {
	of.exists() && count(db, of).is_ok()
}

pub(super) fn count(db: &duckdb::Connection, of: &Path) -> hmerr::Result<u64> {
	let count: i64 = db.query_row(
		&format!(
			r"select count(*)::bigint from read_parquet('{of}')",
			of = of.display()
		),
		[],
		|row| row.get(0),
	)?;

	Ok(u64::try_from(count).unwrap_or_default())
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::*;

	fn scratch(name: &str) -> (duckdb::Connection, PathBuf) {
		let work = std::env::temp_dir().join(format!("declarative_listen_query_{name}"));
		let _ = fs::remove_dir_all(&work);
		let _ = fs::create_dir_all(&work);

		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		(db, work)
	}

	#[test]
	fn a_copy_lands_under_its_final_name() {
		let (db, work) = scratch("copy");
		let into = work.join("out.parquet");

		let _ = copy(&db, &into, "select 1 as one");

		assert!(into.exists());
		assert_eq!(count(&db, &into).unwrap_or_default(), 1);
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_copy_leaves_no_half_written_file_behind() {
		let (db, work) = scratch("atomic");
		let into = work.join("out.parquet");

		let _ = copy(&db, &into, "select 1 as one");

		let left: Vec<PathBuf> = fs::read_dir(&work)
			.into_iter()
			.flatten()
			.filter_map(Result::ok)
			.map(|entry| entry.path())
			.collect();

		assert_eq!(
			left,
			vec![into],
			"a partial parquet must never survive a finished copy"
		);
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_finished_copy_counts_as_done() {
		let (db, work) = scratch("done");
		let into = work.join("out.parquet");
		let _ = copy(&db, &into, "select 1 as one");

		assert!(done(&db, &into));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_truncated_parquet_is_not_done_so_a_resume_rebuilds_it() {
		let (db, work) = scratch("truncated");
		let empty = work.join("empty.parquet");
		let garbage = work.join("garbage.parquet");
		let _ = fs::write(&empty, b"");
		let _ = fs::write(&garbage, b"not a parquet footer");

		assert!(
			!done(&db, &empty),
			"an interrupted write must not look finished"
		);
		assert!(!done(&db, &garbage));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_missing_file_is_not_done() {
		let (db, work) = scratch("absent");

		assert!(!done(&db, &work.join("nothing.parquet")));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_failed_copy_never_publishes_the_final_name() {
		let (db, work) = scratch("failed");
		let into = work.join("out.parquet");

		assert!(copy(&db, &into, "select nonexistent_column").is_err());
		assert!(
			!into.exists(),
			"a name that exists is taken as finished work, so it must not appear on failure"
		);
		let _ = fs::remove_dir_all(&work);
	}
}

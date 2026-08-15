use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

use super::super::{
	open::{self, BUCKET},
	partial, progress,
};

const MEMORY_LIMIT: &str = "5GB";
const THREAD: usize = 3;
const BATCH: usize = 16;

pub(super) struct Scan {
	pub db: duckdb::Connection,
	pub work: PathBuf,
	pub shard: Vec<String>,
}

impl Scan {
	pub(super) fn of(work: &Path, dump: &Path) -> hmerr::Result<Self> {
		let db = duckdb::Connection::open_in_memory()?;

		db.execute_batch(&format!(
			r"
set memory_limit='{MEMORY_LIMIT}';
set threads={THREAD};
set temp_directory='{work}/spill';
set preserve_insertion_order=false;
",
			work = work.display()
		))?;

		Ok(Self {
			db,
			work: work.to_path_buf(),
			shard: shard(dump)?,
		})
	}

	pub(super) fn copy(&self, into: &Path, select: &str) -> hmerr::Result<()> {
		partial::write(into, |partial| {
			self.db.execute_batch(&format!(
				r"
copy ({select}) to '{partial}' (format parquet, compression zstd);
",
				partial = partial.display()
			))?;

			Ok(())
		})
	}

	pub(super) fn batched(
		&self,
		title: &str,
		query: &dyn Fn(&str) -> String,
	) -> hmerr::Result<PathBuf> {
		let partial = self.work.join(title);
		fs::create_dir_all(&partial).map_err(|e| ioe!(partial.to_string_lossy(), e))?;

		let per = self.shard.len().div_ceil(BATCH);
		let bar = progress::step_bar(BATCH as u64, title)?;

		for step in 0..BATCH {
			let into = partial.join(format!("{step}.parquet"));
			let chunk: Vec<&String> = self.shard.iter().skip(step * per).take(per).collect();

			if !chunk.is_empty() && !self.done(&into) {
				self.copy(&into, &query(&quoted(&chunk)))?;
			}

			bar.inc(1);
		}

		bar.finish();

		Ok(partial)
	}

	pub(super) fn bucketed(
		&self,
		into: &Path,
		title: &str,
		query: &dyn Fn(u32) -> String,
	) -> hmerr::Result<()> {
		fs::create_dir_all(into).map_err(|e| ioe!(into.to_string_lossy(), e))?;

		let bar = progress::step_bar(u64::from(BUCKET), title)?;

		for bucket in 0..BUCKET {
			let shard = into.join(open::shard(bucket));

			if !self.done(&shard) {
				self.copy(&shard, &query(bucket))?;
			}

			bar.inc(1);
		}

		bar.finish();

		Ok(())
	}

	pub(super) fn done(&self, of: &Path) -> bool {
		of.exists() && self.count(of).is_ok()
	}

	pub(super) fn count(&self, of: &Path) -> hmerr::Result<u64> {
		let count: i64 = self.db.query_row(
			&format!(
				r"select count(*)::bigint from read_parquet('{of}')",
				of = of.display()
			),
			[],
			|row| row.get(0),
		)?;

		Ok(u64::try_from(count).unwrap_or_default())
	}
}

fn shard(dump: &Path) -> hmerr::Result<Vec<String>> {
	let read = fs::read_dir(dump).map_err(|e| ioe!(dump.to_string_lossy(), e))?;

	let mut shard: Vec<String> = read
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.extension().is_some_and(|ext| ext == "parquet"))
		.map(|path| path.to_string_lossy().to_string())
		.collect();

	if shard.is_empty() {
		return Err(ge!(format!(
			"{R}no parquet shard under {B}{}{D}",
			dump.display()
		))
		.into());
	}

	shard.sort();

	Ok(shard)
}

fn quoted(shard: &[&String]) -> String {
	format!(
		"[{}]",
		shard
			.iter()
			.map(|path| format!("'{path}'"))
			.collect::<Vec<_>>()
			.join(",")
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_shard_list_becomes_a_duckdb_array() {
		let a = "/dump/0.parquet".to_string();
		let b = "/dump/1.parquet".to_string();

		assert_eq!(
			quoted(&[&a, &b]),
			"['/dump/0.parquet','/dump/1.parquet']".to_string()
		);
	}

	#[test]
	fn a_single_shard_still_becomes_an_array() {
		let only = "/dump/0.parquet".to_string();

		assert_eq!(quoted(&[&only]), "['/dump/0.parquet']".to_string());
	}

	fn scan(name: &str) -> (Scan, PathBuf) {
		let work = std::env::temp_dir().join(format!("declarative_listen_scan_{name}"));
		let _ = fs::remove_dir_all(&work);
		let _ = fs::create_dir_all(&work);

		let db = duckdb::Connection::open_in_memory().unwrap_or_else(|_| unreachable!());

		(
			Scan {
				db,
				work: work.clone(),
				shard: Vec::new(),
			},
			work,
		)
	}

	#[test]
	fn a_copy_lands_under_its_final_name() {
		let (scan, work) = scan("copy");
		let into = work.join("out.parquet");

		let _ = scan.copy(&into, "select 1 as one");

		assert!(into.exists());
		assert_eq!(scan.count(&into).unwrap_or_default(), 1);
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_copy_leaves_no_half_written_file_behind() {
		let (scan, work) = scan("atomic");
		let into = work.join("out.parquet");

		let _ = scan.copy(&into, "select 1 as one");

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
		let (scan, work) = scan("done");
		let into = work.join("out.parquet");
		let _ = scan.copy(&into, "select 1 as one");

		assert!(scan.done(&into));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_truncated_parquet_is_not_done_so_a_resume_rebuilds_it() {
		let (scan, work) = scan("truncated");
		let empty = work.join("empty.parquet");
		let garbage = work.join("garbage.parquet");
		let _ = fs::write(&empty, b"");
		let _ = fs::write(&garbage, b"not a parquet footer");

		assert!(
			!scan.done(&empty),
			"an interrupted write must not look finished"
		);
		assert!(!scan.done(&garbage));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_missing_file_is_not_done() {
		let (scan, work) = scan("absent");

		assert!(!scan.done(&work.join("nothing.parquet")));
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_failed_copy_never_publishes_the_final_name() {
		let (scan, work) = scan("failed");
		let into = work.join("out.parquet");

		assert!(scan.copy(&into, "select nonexistent_column").is_err());
		assert!(
			!into.exists(),
			"a name that exists is taken as finished work, so it must not appear on failure"
		);
		let _ = fs::remove_dir_all(&work);
	}

	#[test]
	fn a_directory_with_no_parquet_is_refused() {
		let dir = std::env::temp_dir().join("declarative_listen_scan_empty");
		let _ = fs::create_dir_all(&dir);

		assert!(shard(&dir).is_err());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn only_parquet_counts_as_a_shard_and_the_order_is_stable() {
		let dir = std::env::temp_dir().join("declarative_listen_scan_mixed");
		let _ = fs::create_dir_all(&dir);
		for name in ["1.parquet", "0.parquet", "TIMESTAMP", "COPYING"] {
			let _ = fs::write(dir.join(name), b"");
		}

		let shard = shard(&dir).unwrap_or_default();

		assert_eq!(shard.len(), 2, "{shard:?}");
		assert!(shard[0].ends_with("0.parquet"), "{shard:?}");
		assert!(shard[1].ends_with("1.parquet"), "{shard:?}");
		let _ = fs::remove_dir_all(&dir);
	}
}

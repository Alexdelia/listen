use std::{fs, path::Path};

use ansi::abbrev::{B, D, R};
use hmerr::{ge, ioe};

pub(super) struct Shard {
	pub path: Vec<String>,
	pub bytes: u64,
}

pub(super) fn of(dump: &Path) -> hmerr::Result<Shard> {
	let read = fs::read_dir(dump).map_err(|e| ioe!(dump.to_string_lossy(), e))?;

	let mut found: Vec<(String, u64)> = read
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.extension().is_some_and(|ext| ext == "parquet"))
		.map(|path| {
			let bytes = fs::metadata(&path).map_or(0, |meta| meta.len());

			(path.to_string_lossy().to_string(), bytes)
		})
		.collect();

	if found.is_empty() {
		return Err(ge!(format!(
			"{R}no parquet shard under {B}{}{D}",
			dump.display()
		))
		.into());
	}

	found.sort();

	Ok(Shard {
		bytes: found.iter().map(|(_, bytes)| bytes).sum(),
		path: found.into_iter().map(|(path, _)| path).collect(),
	})
}

pub(super) fn quoted(shard: &[String]) -> String {
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
		let shard = vec!["/dump/0.parquet".to_string(), "/dump/1.parquet".to_string()];

		assert_eq!(
			quoted(&shard),
			"['/dump/0.parquet','/dump/1.parquet']".to_string()
		);
	}

	#[test]
	fn a_single_shard_still_becomes_an_array() {
		assert_eq!(
			quoted(&["/dump/0.parquet".to_string()]),
			"['/dump/0.parquet']".to_string()
		);
	}

	#[test]
	fn a_directory_with_no_parquet_is_refused() {
		let dir = crate::scratch::of("shard", "empty");

		assert!(of(&dir).is_err());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn only_parquet_counts_as_a_shard_and_the_order_is_stable() {
		let dir = crate::scratch::of("shard", "mixed");
		for name in ["1.parquet", "0.parquet", "TIMESTAMP", "COPYING"] {
			let _ = fs::write(dir.join(name), b"");
		}

		let shard = of(&dir).map(|shard| shard.path).unwrap_or_default();

		assert_eq!(shard.len(), 2, "{shard:?}");
		assert!(shard[0].ends_with("0.parquet"), "{shard:?}");
		assert!(shard[1].ends_with("1.parquet"), "{shard:?}");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn how_the_dump_is_sliced_is_read_off_what_the_shards_weigh() {
		let dir = crate::scratch::of("shard", "weight");
		let _ = fs::write(dir.join("0.parquet"), b"payload");
		let _ = fs::write(dir.join("1.parquet"), b"load");

		assert_eq!(of(&dir).map(|shard| shard.bytes).unwrap_or_default(), 11);
		let _ = fs::remove_dir_all(&dir);
	}
}

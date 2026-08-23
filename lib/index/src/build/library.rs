use std::path::{Path, PathBuf};

use super::{
	super::{
		index::{self, layout::BUCKET},
		play,
	},
	scan::Scan,
	stage::Stage,
	work::LIBRARY,
};

pub(super) const BUCKETED: &str = "library_bucket";

pub(super) fn of(scan: &Scan) -> hmerr::Result<PathBuf> {
	let partial = scan.batched(Stage::Library, LIBRARY, &play::counted)?;

	let into = scan.work.join(BUCKETED);

	scan.bucketed(&into, Stage::Compact, &|bucket| {
		format!(
			r"
select user_id, mbid, least(sum(plays), {ceiling})::usmallint as plays
from read_parquet('{partial}/*.parquet')
where user_id % {BUCKET} = {bucket}
group by 1, 2
",
			ceiling = play::CEILING,
			partial = partial.display()
		)
	})?;

	Ok(into)
}

pub(super) fn read(library: &Path) -> String {
	format!(
		"read_parquet('{library}/*.parquet')",
		library = library.display()
	)
}

pub(super) fn read_bucket(library: &Path, bucket: u32) -> String {
	format!(
		"read_parquet('{shard}')",
		shard = library.join(index::layout::shard(bucket)).display()
	)
}

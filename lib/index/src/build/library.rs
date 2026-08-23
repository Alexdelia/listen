use std::path::{Path, PathBuf};

use super::{
	super::open::{self, BUCKET, PLAY_CEILING},
	scan::Scan,
	stage::Stage,
	work::LIBRARY,
};

pub(super) const BUCKETED: &str = "library_bucket";

pub(super) fn of(scan: &Scan) -> hmerr::Result<PathBuf> {
	let partial = scan.batched(Stage::Library, LIBRARY, &|shard| {
		format!(
			r"
select
	l.user_id::uinteger as user_id,
	l.recording_mbid::uuid as mbid,
	least(count(*), {PLAY_CEILING})::usmallint as plays
from read_parquet({shard}) l
where l.recording_mbid is not null
group by 1, 2
"
		)
	})?;

	let into = scan.work.join(BUCKETED);

	scan.bucketed(&into, Stage::Compact, &|bucket| {
		format!(
			r"
select user_id, mbid, least(sum(plays), {PLAY_CEILING})::usmallint as plays
from read_parquet('{partial}/*.parquet')
where user_id % {BUCKET} = {bucket}
group by 1, 2
",
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
		shard = library.join(open::shard(bucket)).display()
	)
}

use std::path::Path;

use super::{
	super::index::layout::USER_STAT, library, pool::Pool, scan::Scan, stage::Stage, work::STAT,
};

pub(super) fn of(scan: &Scan, dir: &Path, library: &Path, pool: &Pool) -> hmerr::Result<()> {
	let partial = scan.work.join(STAT);

	scan.bucketed(&partial, Stage::Stat, &|bucket| {
		crate::user_stat::stat(&pooled(library, bucket, pool))
	})?;

	scan.step(
		Stage::UserStat,
		&dir.join(USER_STAT),
		&format!(
			"select * from read_parquet('{partial}/*.parquet')",
			partial = partial.display()
		),
	)
}

fn pooled(library: &Path, bucket: u32, pool: &Pool) -> String {
	format!(
		"(select l.user_id, l.plays from {library} l \
		semi join {pool} u on u.user_id = l.user_id)",
		library = library::read_bucket(library, bucket),
		pool = pool.read()
	)
}

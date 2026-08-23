use std::path::Path;

use ansi::abbrev::{D, F};

use super::{
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
			stat = stat(&format!(
				"read_parquet('{dir}/{USER_LISTEN}/*.parquet')",
				dir = dir.display()
			)),
			partial = partial.display()
		))?;

		Ok(())
	})
}

const LOW_QUANTILE: f32 = 0.05;
const CENTER_QUANTILE: f32 = 0.5;
const HIGH_QUANTILE: f32 = 0.99;

#[must_use]
pub fn stat(source: &str) -> String {
	format!(
		r"
with occurrence as (
	select user_id, plays, count(*)::bigint as recording
	from {source}
	group by 1, 2
),
whole as (
	select user_id, sum(recording) as recording, sum(plays::bigint * recording) as listen
	from occurrence
	group by 1
),
running as (
	select user_id,
		ln(greatest(plays, 1)) as x,
		sum(plays::bigint * recording) over w as reached
	from occurrence
	window w as (partition by user_id order by plays rows between unbounded preceding and current row)
)
select
	w.user_id::uinteger as user_id,
	min(case when r.reached >= {CENTER_QUANTILE} * w.listen then r.x end)::float as center,
	min(case when r.reached >= {LOW_QUANTILE} * w.listen then r.x end)::float as low,
	min(case when r.reached >= {HIGH_QUANTILE} * w.listen then r.x end)::float as high,
	w.recording::uinteger as recording
from running r
join whole w using (user_id)
group by 1, 5
"
	)
}

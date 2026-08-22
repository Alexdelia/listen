use std::path::{Path, PathBuf};

use super::{super::open::RECORDING, board::Stage, library, scan::Scan};

pub(super) fn of(scan: &Scan, dir: &Path, library: &Path) -> hmerr::Result<PathBuf> {
	let into = dir.join(RECORDING);

	scan.step(
		Stage::Recording,
		&into,
		&format!(
			r"
select
	(row_number() over (order by mbid) - 1)::uinteger as recording_id,
	mbid
from {library}
group by mbid
",
			library = library::read(library)
		),
	)?;

	Ok(into)
}

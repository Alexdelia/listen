use std::{fs, path::Path};

use hmerr::{ioe, pfe, ple, pwe};

use crate::entry::Entry;

pub fn parse<P>(path: P) -> hmerr::Result<Vec<Entry>>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();

	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	match ron::from_str::<Vec<Entry>>(&content) {
		Ok(list) => Ok(list),
		Err(e) => {
			let index = e.position.line - 1;
			let col = e.position.col - 1;
			let line = content
				.lines()
				.nth(index)
				.map(|l| {
					ple!(
						l,
						i: index,
						w: pwe!((col, 1))
					)
				})
				.unwrap_or_default();

			pfe!(
				e.code.to_string(),
				f: path.to_string_lossy(),
				l: line,
			)?
		}
	}
}

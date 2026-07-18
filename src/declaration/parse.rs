use std::{collections::HashSet, fs, path::Path};

use hmerr::{ioe, pfe, ple, pwe};

use super::Entry;

pub fn parse<P>(path: P) -> hmerr::Result<Vec<Entry>>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();

	let content = fs::read_to_string(path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	match ron::from_str::<Vec<Entry>>(&content) {
		Ok(list) => {
			duplicate(&list, &content, path)?;
			Ok(list)
		}
		Err(e) => {
			let index = e.span.start.line - 1;
			let col = e.span.start.col - 1;
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

fn duplicate(list: &[Entry], content: &str, path: &Path) -> hmerr::Result<()> {
	let mut seen = HashSet::with_capacity(list.len());

	let Some(entry) = list.iter().find(|entry| !seen.insert(entry.s)) else {
		return Ok(());
	};

	let mbid = entry.s.to_string();
	let line = content
		.lines()
		.enumerate()
		.filter(|(_, l)| !l.trim_start().starts_with("//") && l.to_lowercase().contains(&mbid))
		.nth(1)
		.map(|(index, l)| {
			ple!(
				l,
				i: index,
				w: pwe!(mbid.clone())
			)
		})
		.unwrap_or_default();

	pfe!(
		format!("duplicate source {mbid}"),
		f: path.to_string_lossy(),
		l: line,
	)
	.map_err(Into::into)
}

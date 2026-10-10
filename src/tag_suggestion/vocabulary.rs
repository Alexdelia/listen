use std::{
	collections::HashSet,
	fs,
	path::Path,
	time::{Duration, SystemTime},
};

use ansi::abbrev::{B, D, R};
use hmerr::ge;
use listen_cache::text;

const URL: &str = "https://musicbrainz.org/ws/2/genre/all?fmt=txt";
const STALE_AFTER: Duration = Duration::from_hours(30 * 24);

pub(super) fn load() -> hmerr::Result<HashSet<String>> {
	let path = listen_cache::path("genre", "all", "txt")?;

	let cached = text::read(&path)?;
	if let Some(cached) = &cached
		&& !stale(&path)
	{
		return Ok(names(cached));
	}

	match fetch() {
		Ok(fetched) => {
			text::write(&path, &fetched)?;
			Ok(names(&fetched))
		}
		Err(e) => cached.map(|cached| names(&cached)).ok_or(e),
	}
}

fn stale(path: &Path) -> bool {
	fs::metadata(path)
		.and_then(|metadata| metadata.modified())
		.ok()
		.and_then(|modified| SystemTime::now().duration_since(modified).ok())
		.is_none_or(|age| age > STALE_AFTER)
}

fn fetch() -> hmerr::Result<String> {
	Ok(listen_agent::shared()
		.get(URL)
		.call()
		.map_err(|e| ge!(format!("{R}failed to fetch {B}{URL}{D}\n{e}")))?
		.body_mut()
		.read_to_string()
		.map_err(|e| ge!(format!("{R}failed to read {B}{URL}{D}\n{e}")))?)
}

fn names(listed: &str) -> HashSet<String> {
	listed
		.lines()
		.map(|line| line.trim().to_lowercase())
		.filter(|name| !name.is_empty())
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_genre_list_is_read_one_lowercased_name_per_line() {
		let names = names("Vaporwave\n\n  city pop \nJ-Core\n");

		assert_eq!(
			names,
			HashSet::from(["vaporwave", "city pop", "j-core"].map(str::to_string))
		);
	}
}

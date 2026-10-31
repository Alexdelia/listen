use std::cmp::Reverse;

use chrono::NaiveDate;
use miette::{IntoDiagnostic, WrapErr};

use crate::declaration::Source;

use super::jspf::CreatedFor;

const PATCH: &str = "weekly-exploration";
const PLAYLIST_PREFIX: &str = "https://listenbrainz.org/playlist/";

pub(super) const CURRENT_WEEK: usize = 0;
pub(super) const LAST_WEEK: usize = 1;
const KEPT: usize = LAST_WEEK + 1;

pub(super) struct Exploration {
	pub mbid: Source,
	pub week: NaiveDate,
}

pub(super) fn explorations(body: &str) -> miette::Result<Vec<Exploration>> {
	let page = serde_json::from_str::<CreatedFor>(body)
		.into_diagnostic()
		.wrap_err("failed to parse created for playlists")?;

	let mut found = page
		.playlists
		.into_iter()
		.map(|wrapper| wrapper.playlist)
		.filter(|playlist| playlist.source_patch() == Some(PATCH))
		.filter_map(|playlist| {
			Some(Exploration {
				mbid: playlist
					.identifier
					.strip_prefix(PLAYLIST_PREFIX)?
					.parse()
					.ok()?,
				week: playlist.date.date_naive(),
			})
		})
		.collect::<Vec<_>>();

	found.sort_unstable_by_key(|exploration| Reverse(exploration.week));
	found.truncate(KEPT);

	Ok(found)
}

#[cfg(test)]
mod tests {
	use super::{
		super::fixture::{BARE_EXTENSION, CREATED_FOR, date},
		*,
	};

	#[test]
	fn two_newest_weekly_explorations_newest_first() {
		let found = explorations(CREATED_FOR).unwrap_or_default();

		assert_eq!(
			found.iter().map(|e| e.week).collect::<Vec<_>>(),
			vec![date(2026, 7, 28), date(2026, 7, 12)]
		);
		assert_eq!(
			found.iter().map(|e| e.mbid.to_string()).collect::<Vec<_>>(),
			vec![
				"66666666-6666-6666-6666-666666666666",
				"33333333-3333-3333-3333-333333333333"
			]
		);
	}

	#[test]
	fn playlist_without_jspf_extension_does_not_abort_the_page() {
		let found = explorations(BARE_EXTENSION).unwrap_or_default();

		assert_eq!(
			found.iter().map(|e| e.week).collect::<Vec<_>>(),
			vec![date(2026, 7, 28)]
		);
	}
}

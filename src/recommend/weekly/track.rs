use ansi::abbrev::{D, R};
use chrono::NaiveDate;
use hmerr::ge;

use crate::recommend::recommendation::{Origin, Recommendation};

use super::jspf::Wrapper;

const RECORDING_PREFIX: &str = "https://musicbrainz.org/recording/";
const FIRST_POSITION: usize = 0;

pub(super) fn tracks(body: &str, week: NaiveDate) -> hmerr::Result<Vec<Recommendation>> {
	let playlist = serde_json::from_str::<Wrapper>(body)
		.map_err(|e| ge!(format!("{R}failed to parse playlist{D}\n{e}")))?
		.playlist;

	Ok(playlist
		.track
		.into_iter()
		.zip(FIRST_POSITION..)
		.filter_map(|(track, position)| {
			Some(Recommendation {
				mbid: track
					.identifier
					.iter()
					.find_map(|identifier| identifier.strip_prefix(RECORDING_PREFIX))?
					.parse()
					.ok()?,
				origin: Origin::WeeklyExploration { week, position },
			})
		})
		.collect())
}

#[cfg(test)]
mod tests {
	use super::{
		super::fixture::{PLAYLIST, date},
		*,
	};
	use crate::recommend::recommendation::Origin;

	fn position(recommendation: &Recommendation) -> usize {
		recommendation.origin.position()
	}

	#[test]
	fn every_recording_mbid_in_playlist_order() {
		let found = tracks(PLAYLIST, date(2026, 7, 12)).unwrap_or_default();

		assert_eq!(
			found.iter().map(|r| r.mbid.to_string()).collect::<Vec<_>>(),
			vec![
				"5ecaf4e8-c19d-4756-b697-20b8478b0c8c",
				"aaaaaaaa-c19d-4756-b697-20b8478b0c8c"
			]
		);
	}

	#[test]
	fn a_track_without_recording_identifier_keeps_the_playlist_position() {
		let found = tracks(PLAYLIST, date(2026, 7, 12)).unwrap_or_default();

		assert_eq!(found.iter().map(position).collect::<Vec<_>>(), vec![0, 2]);
	}

	#[test]
	fn every_track_carries_the_playlist_week() {
		let found = tracks(PLAYLIST, date(2026, 7, 12)).unwrap_or_default();

		assert_eq!(found.len(), 2);
		assert!(found.iter().all(|r| matches!(
			r.origin,
			Origin::WeeklyExploration { week, .. } if week == date(2026, 7, 12)
		)));
	}
}

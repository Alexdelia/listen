use ansi::abbrev::{D, R};
use chrono::{DateTime, Utc};
use hmerr::ge;
use serde::Deserialize;

use crate::{
	declaration::Source,
	recommend::recommendation::{Origin, Recommendation},
};

pub(super) struct Page {
	pub recommendation: Vec<Recommendation>,
	pub fetched: usize,
	pub total: usize,
}

pub(super) fn page(body: &str, offset: usize) -> hmerr::Result<Page> {
	let payload = serde_json::from_str::<Response>(body)
		.map_err(|e| ge!(format!("{R}failed to parse recommendation{D}\n{e}")))?
		.payload;

	Ok(Page {
		recommendation: payload
			.mbids
			.into_iter()
			.zip(offset..)
			.map(|(entry, position)| Recommendation {
				mbid: entry.recording_mbid,
				origin: Origin::CollaborativeFiltering {
					position,
					score: entry.score,
					latest_listened_at: entry.latest_listened_at,
				},
			})
			.collect(),
		fetched: payload.count,
		total: payload.total_mbid_count,
	})
}

#[derive(Deserialize)]
struct Response {
	payload: Payload,
}

#[derive(Deserialize)]
struct Payload {
	count: usize,
	total_mbid_count: usize,
	mbids: Vec<RankedRecording>,
}

#[derive(Deserialize)]
struct RankedRecording {
	recording_mbid: Source,
	score: f32,
	latest_listened_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
	use super::*;

	const BODY: &str = r#"{
		"payload": {
			"count": 2,
			"entity": "recording",
			"last_updated": 1769000000,
			"mbids": [
				{
					"latest_listened_at": "2026-07-01T10:00:00+00:00",
					"recording_mbid": "5ecaf4e8-c19d-4756-b697-20b8478b0c8c",
					"score": 8.5
				},
				{
					"latest_listened_at": null,
					"recording_mbid": "aaaaaaaa-c19d-4756-b697-20b8478b0c8c",
					"score": 7.25
				}
			],
			"model_id": "model",
			"model_url": "https://example.org",
			"offset": 50,
			"total_mbid_count": 1234,
			"user_id": "rob"
		}
	}"#;

	fn positions(page: &Page) -> Vec<usize> {
		page.recommendation
			.iter()
			.map(|recommendation| recommendation.origin.position())
			.collect()
	}

	#[test]
	fn the_offset_carries_over_into_the_position() {
		let page = page(BODY, 50);

		assert!(page.is_ok());
		assert_eq!(positions(&page.unwrap_or_else(|_| empty())), vec![50, 51]);
	}

	#[test]
	fn the_count_and_the_total_come_from_the_payload() {
		let page = page(BODY, 0).unwrap_or_else(|_| empty());

		assert_eq!(page.fetched, 2);
		assert_eq!(page.total, 1234);
	}

	#[test]
	fn every_recording_keeps_its_score_and_last_listen() {
		let page = page(BODY, 0).unwrap_or_else(|_| empty());

		assert!(matches!(
			page.recommendation.first().map(|r| &r.origin),
			Some(Origin::CollaborativeFiltering {
				score,
				latest_listened_at: Some(_),
				..
			}) if (score - 8.5).abs() < f32::EPSILON
		));
		assert!(matches!(
			page.recommendation.get(1).map(|r| &r.origin),
			Some(Origin::CollaborativeFiltering {
				latest_listened_at: None,
				..
			})
		));
	}

	fn empty() -> Page {
		Page {
			recommendation: Vec::new(),
			fetched: 0,
			total: 0,
		}
	}
}

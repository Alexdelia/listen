use std::collections::HashMap;

use ansi::abbrev::{B, D, R};
use chrono::NaiveDate;
use hmerr::ge;
use musicbrainz_rs::{
	Browse, Fetch,
	entity::{artist::Artist, recording::Recording},
};

use crate::{declaration::Source, music_brainz};

const PAGE: u8 = 100;
const FIRST_MONTH: u32 = 1;
const FIRST_DAY: u32 = 1;

pub(super) struct Catalogue {
	pub artist: String,
	pub recording: Vec<Source>,
	pub released: HashMap<Source, NaiveDate>,
}

pub(super) async fn catalogue(mbid: Source) -> hmerr::Result<Catalogue> {
	let client = music_brainz::client();
	let id = mbid.to_string();

	let artist = match Artist::fetch()
		.id(&id)
		.execute_with_client_async(&client)
		.await
	{
		Ok(artist) if confirmed(&artist.id, &id) => artist,
		Ok(_) => {
			return Err(
				not_an_artist(&client, &id, format!("{R}no artist under {B}{id}{D}")).await,
			);
		}
		Err(e) => {
			return Err(not_an_artist(
				&client,
				&id,
				format!("{R}failed to fetch artist {B}{id}{D}\n{e:#?}"),
			)
			.await);
		}
	};

	let mut recording = Vec::new();
	let mut released = HashMap::new();
	let mut offset = 0;
	loop {
		let page = Recording::browse()
			.by_artist(&id)
			.limit(PAGE)
			.offset(offset)
			.execute_with_client_async(&client)
			.await
			.map_err(|e| {
				ge!(format!(
					"{R}failed to browse the recordings of {B}{name}{D}\n{e:#?}",
					name = artist.name
				))
			})?;

		for entity in &page.entities {
			let Ok(mbid) = entity.id.parse::<Source>() else {
				continue;
			};

			recording.push(mbid);

			if let Some(date) = entity
				.first_release_date
				.as_ref()
				.and_then(|date| release_date(&date.0))
			{
				released.insert(mbid, date);
			}
		}

		offset = offset.saturating_add(u16::from(PAGE));
		if page.entities.is_empty() || i32::from(offset) >= page.count {
			break;
		}
	}

	Ok(Catalogue {
		artist: artist.name,
		recording,
		released,
	})
}

fn release_date(date: &str) -> Option<NaiveDate> {
	let mut part = date.split('-');

	let year = part.next()?.parse().ok()?;
	let month = part.next().and_then(number).unwrap_or(FIRST_MONTH);
	let day = part.next().and_then(number).unwrap_or(FIRST_DAY);

	NaiveDate::from_ymd_opt(year, month, day)
}

fn number(part: &str) -> Option<u32> {
	part.parse().ok()
}

fn confirmed(fetched: &str, id: &str) -> bool {
	fetched == id
}

async fn not_an_artist(
	client: &musicbrainz_rs::MusicBrainzClient,
	id: &str,
	failure: String,
) -> Box<dyn std::error::Error> {
	if Recording::fetch()
		.id(id)
		.execute_with_client_async(client)
		.await
		.is_ok_and(|recording| confirmed(&recording.id, id))
	{
		return ge!(
			format!("{R}{B}{id}{D}{R} is a recording, not an artist{D}"),
			h: "listenbrainz only recommends by listen count for an artist mbid"
		)
		.into();
	}

	ge!(failure).into()
}

#[cfg(test)]
mod tests {
	use super::*;

	const ID: &str = "beff21d3-88c7-4ee0-8b7a-40b6db22c6d7";

	#[test]
	fn an_entity_under_the_asked_mbid_is_confirmed() {
		let artist = Artist {
			id: ID.to_string(),
			name: "Pendulum".to_string(),
			..Default::default()
		};

		assert!(confirmed(&artist.id, ID));
	}

	#[test]
	fn a_musicbrainz_error_payload_confirms_nothing() {
		assert!(!confirmed(&Artist::default().id, ID));
		assert!(!confirmed("", ID));
	}

	#[test]
	fn a_full_date_is_read_as_it_is() {
		assert_eq!(
			release_date("2008-05-09"),
			NaiveDate::from_ymd_opt(2008, 5, 9)
		);
	}

	#[test]
	fn a_partial_date_falls_back_to_the_start_of_what_it_gives() {
		assert_eq!(release_date("2008"), NaiveDate::from_ymd_opt(2008, 1, 1));
		assert_eq!(release_date("2008-05"), NaiveDate::from_ymd_opt(2008, 5, 1));
		assert_eq!(
			release_date("2008-??-09"),
			NaiveDate::from_ymd_opt(2008, 1, 9)
		);
	}

	#[test]
	fn a_date_without_a_year_is_no_date() {
		assert_eq!(release_date(""), None);
		assert_eq!(release_date("????-05-09"), None);
	}

	#[test]
	fn an_impossible_date_is_no_date() {
		assert_eq!(release_date("2008-13-42"), None);
	}
}

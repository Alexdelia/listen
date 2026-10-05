use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{cache, declaration::Source};

use super::{
	diversify::{DIVERSITY, Pick},
	preference::ONE_PLAY,
};

const FILE: &str = "similar.jsonl";

pub(super) fn path() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(FILE))
}

#[derive(Serialize, Deserialize)]
pub(super) struct Entry {
	pub mbid: Source,
	pub raw: f32,
	pub expected: f32,
	pub support: u32,
	pub near: Vec<Source>,
	pub one_play: f32,
	pub diversity: f32,
	pub allow_known_artist: bool,
	pub shown_at: DateTime<Utc>,
}

impl Entry {
	pub(super) fn of(pick: &Pick, allow_known_artist: bool) -> Self {
		Self {
			mbid: pick.mbid,
			raw: pick.raw,
			expected: pick.expected,
			support: pick.support,
			near: pick.near.iter().map(|near| near.mbid).collect(),
			one_play: ONE_PLAY,
			diversity: DIVERSITY,
			allow_known_artist,
			shown_at: Utc::now(),
		}
	}
}

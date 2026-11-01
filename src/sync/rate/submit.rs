use std::fmt::Write;

use crate::music_brainz;

use super::Rating;

pub(super) const CHUNK: usize = 200;

const ENDPOINT: &str = "https://musicbrainz.org/ws/2/rating";

pub(super) fn submit(bearer: &str, rating: &[Rating]) -> miette::Result<()> {
	music_brainz::submit(bearer, ENDPOINT, &recording_list(rating), "rating")
}

fn recording_list(rating: &[Rating]) -> String {
	let mut recording = String::new();
	for (source, value) in rating {
		let _ = write!(
			recording,
			"<recording id=\"{source}\"><user-rating>{value}</user-rating></recording>",
		);
	}

	recording
}

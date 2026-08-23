use musicbrainz_rs::entity::recording::Recording;

use crate::streaming_source::{self, StreamingSource};

pub(super) enum Streaming {
	SoundCloud,
	Bandcamp,
	YouTubeMusic(String),
}

impl Streaming {
	const fn priority(&self) -> u8 {
		match self {
			Self::SoundCloud => StreamingSource::SoundCloud.priority(),
			Self::Bandcamp => StreamingSource::Bandcamp.priority(),
			Self::YouTubeMusic(_) => StreamingSource::YouTubeMusic.priority(),
		}
	}

	pub(super) const fn name(&self) -> &'static str {
		match self {
			Self::SoundCloud => "soundcloud",
			Self::Bandcamp => "bandcamp",
			Self::YouTubeMusic(_) => "youtube music",
		}
	}
}

pub(super) fn streaming(recording: &Recording) -> Option<Streaming> {
	streaming_source::streaming_url(recording)
		.filter_map(classify)
		.min_by_key(Streaming::priority)
}

fn classify(url: &str) -> Option<Streaming> {
	match StreamingSource::try_from(url).ok()? {
		StreamingSource::SoundCloud => Some(Streaming::SoundCloud),
		StreamingSource::Bandcamp => Some(Streaming::Bandcamp),
		StreamingSource::YouTubeMusic => video_id(url).map(Streaming::YouTubeMusic),
		StreamingSource::YouTube => None,
	}
}

pub(super) fn video_id(url: &str) -> Option<String> {
	let query = url
		.strip_prefix(StreamingSource::YouTubeMusic.base_url().as_str())?
		.strip_prefix("/watch?")?;

	query
		.split('&')
		.find_map(|param| param.strip_prefix("v="))
		.map(str::to_string)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn recording(relation: &str) -> Recording {
		serde_json::from_str(&format!(
			r#"{{
				"id": "6a04afe4-cf2f-4052-a1e5-b6eea14eaefd",
				"title": "Distortion!!",
				"length": 203000,
				"video": false,
				"disambiguation": "",
				"relations": [{relation}]
			}}"#
		))
		.unwrap()
	}

	fn url_relation(relation_type: &str, resource: &str) -> String {
		ended_url_relation(relation_type, resource, false)
	}

	fn ended_url_relation(relation_type: &str, resource: &str, ended: bool) -> String {
		format!(
			r#"{{
				"type": "{relation_type}",
				"type-id": "00000000-0000-0000-0000-000000000000",
				"direction": "forward",
				"target-type": "url",
				"ended": {ended},
				"url": {{
					"id": "11111111-1111-1111-1111-111111111111",
					"resource": "{resource}"
				}}
			}}"#
		)
	}

	#[test]
	fn a_free_streaming_youtube_music_link_is_picked_up() {
		let recording = recording(&url_relation(
			"free streaming",
			"https://music.youtube.com/watch?v=YiMJM0Bthv4",
		));

		assert!(matches!(
			streaming(&recording),
			Some(Streaming::YouTubeMusic(id)) if id == "YiMJM0Bthv4"
		));
	}

	#[test]
	fn a_paid_streaming_youtube_music_link_is_picked_up_too() {
		let recording = recording(&url_relation(
			"streaming",
			"https://music.youtube.com/watch?v=YiMJM0Bthv4",
		));

		assert!(matches!(
			streaming(&recording),
			Some(Streaming::YouTubeMusic(id)) if id == "YiMJM0Bthv4"
		));
	}

	#[test]
	fn a_paid_streaming_soundcloud_link_is_picked_up_too() {
		let recording = recording(&url_relation(
			"streaming",
			"https://soundcloud.com/artist/track",
		));

		assert!(matches!(streaming(&recording), Some(Streaming::SoundCloud)));
	}

	#[test]
	fn a_plain_youtube_link_is_not_a_streaming_source() {
		let recording = recording(&url_relation(
			"free streaming",
			"https://www.youtube.com/watch?v=Xy6lZxoJ4ts",
		));

		assert!(streaming(&recording).is_none());
	}

	#[test]
	fn an_unrelated_relation_type_is_ignored() {
		let recording = recording(&url_relation(
			"purchase for download",
			"https://music.youtube.com/watch?v=YiMJM0Bthv4",
		));

		assert!(streaming(&recording).is_none());
	}

	#[test]
	fn an_ended_streaming_link_is_ignored() {
		let recording = recording(&ended_url_relation(
			"free streaming",
			"https://music.youtube.com/watch?v=-_jTtUZIufs",
			true,
		));

		assert!(streaming(&recording).is_none());
	}

	#[test]
	fn a_live_streaming_link_wins_over_an_ended_one_of_the_same_source() {
		let recording = recording(&format!(
			"{ended},{live}",
			ended = ended_url_relation(
				"free streaming",
				"https://music.youtube.com/watch?v=-_jTtUZIufs",
				true
			),
			live = url_relation(
				"free streaming",
				"https://music.youtube.com/watch?v=LaZIDFaobMU"
			),
		));

		assert!(matches!(
			streaming(&recording),
			Some(Streaming::YouTubeMusic(id)) if id == "LaZIDFaobMU"
		));
	}

	#[test]
	fn a_free_streaming_bandcamp_track_is_picked_up() {
		let recording = recording(&url_relation(
			"free streaming",
			"https://swkabc.bandcamp.com/track/bad-apple",
		));

		assert!(matches!(streaming(&recording), Some(Streaming::Bandcamp)));
	}

	#[test]
	fn a_bandcamp_album_is_not_a_streaming_source() {
		let recording = recording(&url_relation(
			"free streaming",
			"https://swkabc.bandcamp.com/album/bad-apple",
		));

		assert!(streaming(&recording).is_none());
	}

	#[test]
	fn youtube_music_wins_over_bandcamp() {
		let recording = recording(&format!(
			"{bandcamp},{youtube}",
			bandcamp = url_relation(
				"free streaming",
				"https://swkabc.bandcamp.com/track/bad-apple"
			),
			youtube = url_relation(
				"free streaming",
				"https://music.youtube.com/watch?v=YiMJM0Bthv4"
			),
		));

		assert!(matches!(
			streaming(&recording),
			Some(Streaming::YouTubeMusic(id)) if id == "YiMJM0Bthv4"
		));
	}

	#[test]
	fn bandcamp_wins_over_plain_youtube() {
		let recording = recording(&format!(
			"{youtube},{bandcamp}",
			youtube = url_relation(
				"free streaming",
				"https://www.youtube.com/watch?v=Xy6lZxoJ4ts"
			),
			bandcamp = url_relation(
				"free streaming",
				"https://swkabc.bandcamp.com/track/bad-apple"
			),
		));

		assert!(matches!(streaming(&recording), Some(Streaming::Bandcamp)));
	}

	#[test]
	fn soundcloud_wins_over_youtube_music_across_relation_types() {
		let recording = recording(&format!(
			"{youtube},{soundcloud}",
			youtube = url_relation(
				"free streaming",
				"https://music.youtube.com/watch?v=YiMJM0Bthv4"
			),
			soundcloud = url_relation("streaming", "https://soundcloud.com/artist/track"),
		));

		assert!(matches!(streaming(&recording), Some(Streaming::SoundCloud)));
	}
}

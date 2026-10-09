use crate::streaming_source::StreamingSource;

pub(super) fn sort<F>(urls: &mut [(StreamingSource, String)], is_song: F)
where
	F: Fn(&str) -> bool,
{
	let youtube_music_to_choose_from =
		urls.iter()
			.filter(|(source, _)| is_youtube_music(source))
			.count() > 1;

	urls.sort_by_cached_key(|(source, url)| {
		let video = youtube_music_to_choose_from && is_youtube_music(source) && !is_song(url);

		(source.priority(), video)
	});
}

const fn is_youtube_music(source: &StreamingSource) -> bool {
	matches!(source, StreamingSource::YouTubeMusic)
}

#[cfg(test)]
mod tests {
	use super::*;

	const SONG: &str = "https://music.youtube.com/watch?v=C6NOCEYCv3M";
	const VIDEO: &str = "https://music.youtube.com/watch?v=QKdZDgHOmog";
	const SOUNDCLOUD: &str = "https://soundcloud.com/artist/track";

	fn urls(listed: &[&str]) -> Vec<(StreamingSource, String)> {
		listed
			.iter()
			.map(|url| (StreamingSource::try_from(*url).unwrap(), (*url).to_string()))
			.collect()
	}

	fn sorted(listed: &[&str]) -> Vec<String> {
		let mut urls = urls(listed);
		sort(&mut urls, |url| url == SONG);
		urls.into_iter().map(|(_, url)| url).collect()
	}

	#[test]
	fn a_song_is_tried_before_a_video_musicbrainz_lists_ahead_of_it() {
		assert_eq!(sorted(&[VIDEO, SONG]), [SONG, VIDEO]);
	}

	#[test]
	fn a_song_already_listed_first_stays_first() {
		assert_eq!(sorted(&[SONG, VIDEO]), [SONG, VIDEO]);
	}

	#[test]
	fn the_host_priority_still_outranks_being_a_song() {
		assert_eq!(
			sorted(&[VIDEO, SONG, SOUNDCLOUD]),
			[SOUNDCLOUD, SONG, VIDEO]
		);
	}

	#[test]
	fn a_lone_youtube_music_link_is_never_probed() {
		let mut urls = urls(&[VIDEO, SOUNDCLOUD]);
		sort(&mut urls, |_| unreachable!());

		let order: Vec<_> = urls.into_iter().map(|(_, url)| url).collect();
		assert_eq!(order, [SOUNDCLOUD, VIDEO]);
	}
}

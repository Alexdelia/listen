use const_format::concatcp;

pub(super) const HOST: &str = "bandcamp.com";

const ARTIST_HOST_SUFFIX: &str = concatcp!(".", HOST);
const TRACK_PATH: &str = "track/";

pub(super) fn is_track(url: &str) -> bool {
	let Some(rest) = url.strip_prefix("https://") else {
		return false;
	};
	let Some((host, path)) = rest.split_once('/') else {
		return false;
	};

	host.ends_with(ARTIST_HOST_SUFFIX) && path.starts_with(TRACK_PATH)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_artist_track_is_a_track() {
		assert!(is_track("https://swkabc.bandcamp.com/track/bad-apple"));
	}

	#[test]
	fn an_album_is_not_a_track() {
		assert!(!is_track("https://swkabc.bandcamp.com/album/touhou-remix"));
	}

	#[test]
	fn an_artist_page_is_not_a_track() {
		assert!(!is_track("https://swkabc.bandcamp.com/"));
	}

	#[test]
	fn the_bare_domain_is_not_a_track() {
		assert!(!is_track("https://bandcamp.com/discover/electronic"));
	}

	#[test]
	fn a_lookalike_domain_is_not_a_track() {
		assert!(!is_track("https://notbandcamp.com/track/bad-apple"));
	}

	#[test]
	fn http_is_not_a_track() {
		assert!(!is_track("http://swkabc.bandcamp.com/track/bad-apple"));
	}
}

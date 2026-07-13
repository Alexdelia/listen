use super::super::verify::Info;
use super::text::normalize_title;

const DURATION_TOLERANCE_SEC: i64 = 1;

pub(super) fn is_match(accepted_title: &[String], length: i64, info: &Info) -> bool {
	if !info.is_song() {
		return false;
	}

	let Some(track) = info.track.as_deref() else {
		return false;
	};
	if !title_match(accepted_title, track) {
		return false;
	}

	let Some(duration) = info.duration else {
		return false;
	};

	(length - duration).abs() <= DURATION_TOLERANCE_SEC
}

fn title_match(accepted_title: &[String], track: &str) -> bool {
	let track = normalize_title(track);
	if track.is_empty() {
		return false;
	}

	accepted_title
		.iter()
		.any(|t| !t.is_empty() && (track.starts_with(t) || t.starts_with(&track)))
}

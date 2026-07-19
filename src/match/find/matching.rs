use super::super::verify::Info;
use super::text::normalize_title;

const DURATION_TOLERANCE_SEC: i64 = 1;

pub(super) enum Mismatch {
	NotSong(Option<String>),
	NoTitle,
	Title,
	NoDuration,
	Duration(i64),
}

impl Mismatch {
	pub(super) fn rank(&self) -> (u8, i64) {
		match self {
			Self::Duration(delta) => (0, delta.abs()),
			Self::NoDuration => (1, 0),
			Self::Title => (2, 0),
			Self::NoTitle => (3, 0),
			Self::NotSong(_) => (4, 0),
		}
	}
}

pub(super) fn check(accepted_title: &[String], length: i64, info: &Info) -> Option<Mismatch> {
	if !info.is_song() {
		return Some(Mismatch::NotSong(info.media_type.clone()));
	}

	let Some(track) = info.track.as_deref() else {
		return Some(Mismatch::NoTitle);
	};
	if !title_match(accepted_title, track) {
		return Some(Mismatch::Title);
	}

	let Some(duration) = info.duration else {
		return Some(Mismatch::NoDuration);
	};

	let delta = duration - length;
	(delta.abs() > DURATION_TOLERANCE_SEC).then_some(Mismatch::Duration(delta))
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

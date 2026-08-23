use listen_index::own::{self, Gap};

use super::{super::fetch::ListenCount, Held};

pub(super) const MBID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

pub(super) const DUMP: &str = "2026-07-12 00:00:04.001868+00:00";
pub(super) const NEWER: &str = "2026-08-16 00:00:03.000000+00:00";
pub(super) const LATEST: &str = "2026-08-22 00:00:02.641933+00:00";

pub(super) fn held() -> Held {
	Held {
		dump: DUMP.to_string(),
		reached: String::new(),
		gap: Vec::new(),
		covered: 1_783_814_404,
		count: ListenCount::new(),
		fold: Some(ListenCount::new()),
	}
}

pub(super) fn play(mbid: &str, plays: u32) -> own::Play {
	own::Play {
		mbid: mbid.parse().unwrap_or_default(),
		plays,
		track: "Fairy Dance".to_string(),
		artist: "UNDEAD CORPORATION".to_string(),
	}
}

pub(super) fn fold(reached: &str, plays: u32, gap: Vec<Gap>) -> own::Fold {
	own::Fold {
		reached: reached.to_string(),
		covered: 0,
		play: vec![play(MBID, plays)],
		gap,
	}
}

pub(super) fn plays(held: &Held, mbid: &str) -> Option<u32> {
	held.counted()
		.get(&mbid.parse().unwrap_or_default())
		.map(|listen| listen.count)
}

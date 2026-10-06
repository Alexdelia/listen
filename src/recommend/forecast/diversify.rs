use crate::declaration::Source;

use super::{
	super::{
		feed,
		labelled::Labelled,
		log::Log,
		recommendation::{self, Origin, Recommendation},
		skip::Skip,
	},
	log,
	profile::{Near, Profile},
};

pub(super) const DIVERSITY: f32 = 20.0;

pub(super) struct Pick {
	pub mbid: Source,
	pub raw: f32,
	pub expected: f32,
	pub support: u32,
	pub profile: Profile,
	pub near: Vec<Near>,
}

pub(super) struct Stream {
	pick: Vec<Pick>,
	served: Vec<Profile>,
	position: usize,
	allow_known_artist: bool,
	log: Log,
}

pub(super) const fn stream(pick: Vec<Pick>, allow_known_artist: bool, log: Log) -> Stream {
	Stream {
		pick,
		served: Vec::new(),
		position: 0,
		allow_known_artist,
		log,
	}
}

impl feed::Feed for Stream {
	fn next(&mut self, skip: &Skip) -> hmerr::Result<Option<Recommendation>> {
		self.pick.retain(|pick| !skip.seen(pick.mbid));

		let Some(judged) = self.choose() else {
			return Ok(None);
		};
		let pick = self.pick.swap_remove(judged.index);

		self.log
			.append(&log::Entry::of(&pick, self.allow_known_artist))?;

		let position = self.position;
		self.position += 1;

		let recommendation = Recommendation {
			mbid: pick.mbid,
			origin: Origin::Forecast {
				expected: pick.expected,
				raw: pick.raw,
				support: pick.support,
				near: pick
					.near
					.iter()
					.map(|near| recommendation::Near {
						recording: Labelled(near.mbid),
						q: near.q,
					})
					.collect(),
				position,
				worth: judged.worth,
				redundancy: judged.redundancy,
			},
		};
		self.served.push(pick.profile);

		Ok(Some(recommendation))
	}
}

struct Judged {
	index: usize,
	worth: f32,
	redundancy: f32,
}

impl Stream {
	fn choose(&self) -> Option<Judged> {
		self.pick
			.iter()
			.enumerate()
			.map(|(index, pick)| {
				let redundancy = self.redundancy(pick);

				Judged {
					index,
					worth: DIVERSITY.mul_add(-redundancy, pick.expected),
					redundancy,
				}
			})
			.max_by(|a, b| a.worth.total_cmp(&b.worth))
	}

	fn redundancy(&self, pick: &Pick) -> f32 {
		self.served
			.iter()
			.map(|served| pick.profile.cosine(served))
			.fold(0.0, f32::max)
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::{super::super::feed::Feed, *};

	fn pick(byte: u8, expected: f32, seed: u32) -> Pick {
		Pick {
			mbid: Source::from_bytes([byte; 16]),
			raw: expected,
			expected,
			support: 5,
			profile: Profile::of_strength([(seed, 1.0)]),
			near: Vec::new(),
		}
	}

	fn log(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("declarative_listen_forecast_{name}.jsonl"));
		let _ = fs::remove_file(&path);
		path
	}

	fn drain(stream: &mut Stream, skip: &Skip) -> Vec<u8> {
		let mut served = Vec::new();
		while let Ok(Some(recommendation)) = stream.next(skip) {
			served.push(recommendation.mbid.as_bytes()[0]);
		}
		served
	}

	#[test]
	fn the_best_expected_rating_is_served_first() {
		let mut stream = stream(vec![pick(1, 60.0, 0), pick(2, 80.0, 1)], false, Log::Off);

		assert_eq!(drain(&mut stream, &Skip::default()).first(), Some(&2));
	}

	#[test]
	fn a_near_copy_of_what_was_served_waits_behind_another_region() {
		let mut stream = stream(
			vec![pick(1, 80.0, 0), pick(2, 75.0, 0), pick(3, 70.0, 1)],
			false,
			Log::Off,
		);

		assert_eq!(drain(&mut stream, &Skip::default()), vec![1, 3, 2]);
	}

	#[test]
	fn a_much_better_near_copy_still_beats_a_weak_other_region() {
		let mut stream = stream(
			vec![pick(1, 90.0, 0), pick(2, 88.0, 0), pick(3, 60.0, 1)],
			false,
			Log::Off,
		);

		assert_eq!(drain(&mut stream, &Skip::default()), vec![1, 2, 3]);
	}

	#[test]
	fn a_pick_declared_or_declined_meanwhile_is_never_served() {
		let mut skip = Skip::default();
		skip.fresh(Source::from_bytes([1; 16]));
		let mut stream = stream(vec![pick(1, 80.0, 0), pick(2, 70.0, 1)], false, Log::Off);

		assert_eq!(drain(&mut stream, &skip), vec![2]);
	}

	#[test]
	fn a_pool_with_nothing_left_ends_the_feed() {
		let mut skip = Skip::default();
		skip.fresh(Source::from_bytes([1; 16]));
		let mut stream = stream(vec![pick(1, 80.0, 0)], false, Log::Off);

		assert!(matches!(stream.next(&skip), Ok(None)));
	}

	#[test]
	fn every_served_pick_is_logged_with_what_scored_it() {
		let path = log("logged");
		let mut stream = stream(
			vec![pick(1, 80.0, 0), pick(2, 70.0, 1)],
			true,
			Log::File(path.clone()),
		);

		drain(&mut stream, &Skip::default());
		let content = fs::read_to_string(&path).unwrap_or_default();

		assert_eq!(content.lines().count(), 2);
		assert!(content.contains("\"expected\":80"), "{content}");
		assert!(content.contains("\"allow_known_artist\":true"), "{content}");
		let _ = fs::remove_file(path);
	}

	fn redundancy(recommendation: &Recommendation) -> Option<(f32, f32)> {
		match recommendation.origin {
			Origin::Forecast {
				worth, redundancy, ..
			} => Some((worth, redundancy)),
			_ => None,
		}
	}

	#[test]
	fn a_pick_carries_the_redundancy_it_was_chosen_with() {
		let mut stream = stream(
			vec![pick(1, 80.0, 0), pick(2, 75.0, 0), pick(3, 70.0, 1)],
			false,
			Log::Off,
		);
		let mut chosen = Vec::new();
		while let Ok(Some(recommendation)) = stream.next(&Skip::default()) {
			chosen.extend(redundancy(&recommendation));
		}

		let (worth, redundant) = chosen.last().copied().unwrap_or_default();

		assert!((redundant - 1.0).abs() < 1e-4, "{redundant}");
		assert!((worth - 55.0).abs() < 1e-3, "{worth}");
	}
}

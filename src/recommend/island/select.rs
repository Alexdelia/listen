use std::{collections::VecDeque, path::PathBuf};

use chrono::Utc;
use hmerr::ioe;

use super::super::{
	feed,
	recommendation::{Origin, Recommendation},
};
use super::{log, score::Candidate};

const WIDE_QUOTA: usize = 3;
const NARROW_QUOTA: usize = 2;
const WIDE_ISLAND: usize = 6;

pub(super) struct Island {
	pub name: String,
	pub member: usize,
}

pub(super) struct Stream {
	island: Vec<Island>,
	candidate: Vec<VecDeque<Candidate>>,
	turn: usize,
	spent: usize,
	served: usize,
	stay: bool,
	ask: bool,
	popularity_damp: f32,
	granularity: f64,
	log: PathBuf,
}

pub(super) fn stream(
	island: Vec<Island>,
	candidate: Vec<Vec<Candidate>>,
	ask: bool,
	popularity_damp: f32,
	granularity: f64,
	log: PathBuf,
) -> Stream {
	Stream {
		island,
		candidate: candidate.into_iter().map(VecDeque::from).collect(),
		turn: 0,
		spent: 0,
		served: 0,
		stay: true,
		ask,
		popularity_damp,
		granularity,
		log,
	}
}

impl feed::Feed for Stream {
	fn next(&mut self) -> hmerr::Result<Option<Recommendation>> {
		if self.drained() {
			return Ok(None);
		}

		self.decide()?;

		let Some(candidate) = self
			.candidate
			.get_mut(self.turn)
			.and_then(VecDeque::pop_front)
		else {
			return Ok(None);
		};

		let name = self
			.island
			.get(self.turn)
			.map(|island| island.name.clone())
			.unwrap_or_default();
		let member = self.island.get(self.turn).map_or(0, |island| island.member);

		log::append(
			&self.log,
			&log::Entry {
				mbid: candidate.mbid,
				island: name.clone(),
				member,
				score: candidate.score,
				backer: candidate.backer,
				plays: candidate.plays,
				popularity_damp: self.popularity_damp,
				granularity: self.granularity,
				stay: self.stay,
				shown_at: Utc::now(),
			},
		)?;

		let position = self.served;
		self.served += 1;
		self.spent += 1;

		Ok(Some(Recommendation {
			mbid: candidate.mbid,
			origin: Origin::Island {
				name,
				member,
				score: candidate.score,
				backer: candidate.backer,
				plays: candidate.plays,
				position,
			},
		}))
	}
}

impl Stream {
	fn decide(&mut self) -> hmerr::Result<()> {
		if self.served == 0 {
			self.turn = self.living(self.turn);
			self.stay = true;

			return Ok(());
		}

		if self.ask {
			self.stay = self.alive(self.turn) && self.asked()?;

			if !self.stay {
				self.turn = self.living(self.turn + 1);
			}

			return Ok(());
		}

		self.stay = self.spent < self.quota() && self.alive(self.turn);

		if !self.stay {
			self.spent = 0;
			self.turn = self.living(self.turn + 1);
		}

		Ok(())
	}

	fn asked(&self) -> hmerr::Result<bool> {
		let name = self
			.island
			.get(self.turn)
			.map(|island| island.name.as_str())
			.unwrap_or_default();

		ux::ask_yn(&format!("stay on {name}"), true).map_err(|e| ioe!("stdin", e).into())
	}

	fn quota(&self) -> usize {
		if self.turn < WIDE_ISLAND {
			WIDE_QUOTA
		} else {
			NARROW_QUOTA
		}
	}

	fn alive(&self, turn: usize) -> bool {
		self.candidate
			.get(turn)
			.is_some_and(|candidate| !candidate.is_empty())
	}

	fn living(&self, from: usize) -> usize {
		let count = self.candidate.len().max(1);

		(0..count)
			.map(|step| (from + step) % count)
			.find(|turn| self.alive(*turn))
			.unwrap_or(from % count)
	}

	fn drained(&self) -> bool {
		self.candidate.iter().all(VecDeque::is_empty)
	}
}

#[cfg(test)]
mod tests {
	use super::{super::super::feed::Feed, *};
	use crate::declaration::Source;

	fn candidate(nibble: u8, count: usize) -> Vec<Candidate> {
		(0..count)
			.map(|step| {
				let mut byte = [nibble; 16];
				byte[1] = u8::try_from(step).unwrap_or_default();

				Candidate {
					mbid: Source::from_bytes(byte),
					score: 1.0,
					backer: 5,
					plays: 10,
				}
			})
			.collect()
	}

	fn island(count: usize) -> Vec<Island> {
		(0..count)
			.map(|step| Island {
				name: format!("isl{step}"),
				member: 10,
			})
			.collect()
	}

	fn quiet(candidate: Vec<Vec<Candidate>>) -> Stream {
		let island = island(candidate.len());
		let log = std::env::temp_dir().join(format!(
			"declarative_listen_select_{}.jsonl",
			candidate.iter().map(Vec::len).sum::<usize>()
		));
		let _ = std::fs::remove_file(&log);

		stream(island, candidate, false, 0.6, 1.0, log)
	}

	fn drain(stream: &mut Stream, take: usize) -> Vec<u8> {
		let mut seen = Vec::new();

		for _ in 0..take {
			match stream.next() {
				Ok(Some(recommendation)) => seen.push(recommendation.mbid.as_bytes()[0]),
				_ => break,
			}
		}

		seen
	}

	#[test]
	fn the_top_island_spends_its_whole_quota_before_the_next_one() {
		let mut stream = quiet(vec![candidate(1, 5), candidate(2, 5)]);

		assert_eq!(drain(&mut stream, 6), vec![1, 1, 1, 2, 2, 2]);
	}

	fn round() -> usize {
		WIDE_ISLAND * WIDE_QUOTA + (8 - WIDE_ISLAND) * NARROW_QUOTA
	}

	#[test]
	fn a_low_ranked_island_gets_the_narrow_quota_and_a_top_one_the_wide_quota() {
		let candidate: Vec<Vec<Candidate>> = (0..8u8).map(|island| candidate(island, 4)).collect();

		let mut stream = quiet(candidate);
		let seen = drain(&mut stream, round());

		assert_eq!(
			seen.iter().filter(|island| **island == 0).count(),
			WIDE_QUOTA,
			"{seen:?}"
		);
		assert_eq!(
			seen.iter().filter(|island| **island == 7).count(),
			NARROW_QUOTA,
			"{seen:?}"
		);
	}

	#[test]
	fn a_spent_round_starts_another_one_instead_of_ending_the_stream() {
		let candidate: Vec<Vec<Candidate>> = (0..8u8).map(|island| candidate(island, 4)).collect();

		let mut stream = quiet(candidate);

		assert_eq!(drain(&mut stream, round() + 1).len(), round() + 1);
	}

	#[test]
	fn an_empty_island_is_skipped_without_spending_a_turn() {
		let mut stream = quiet(vec![candidate(1, 3), Vec::new(), candidate(3, 3)]);

		assert_eq!(drain(&mut stream, 6), vec![1, 1, 1, 3, 3, 3]);
	}

	#[test]
	fn a_drained_stream_stops() {
		let mut stream = quiet(vec![candidate(1, 2)]);

		assert_eq!(drain(&mut stream, 5).len(), 2);
	}

	#[test]
	fn no_candidate_yields_nothing() {
		let mut stream = quiet(vec![Vec::new(), Vec::new()]);

		assert!(drain(&mut stream, 3).is_empty());
	}

	#[test]
	fn a_single_island_serves_everything_it_has() {
		let mut stream = quiet(vec![candidate(1, 7)]);

		assert_eq!(drain(&mut stream, 9).len(), 7);
	}

	#[test]
	fn the_position_counts_every_recommendation_the_stream_served() {
		let mut stream = quiet(vec![candidate(1, 2), candidate(2, 2)]);
		let mut position = Vec::new();

		while let Ok(Some(recommendation)) = stream.next() {
			position.push(recommendation.origin.position());
		}

		assert_eq!(position, vec![0, 1, 2, 3]);
	}

	#[test]
	fn every_recommendation_carries_the_island_that_produced_it() {
		let mut stream = quiet(vec![candidate(1, 1), candidate(2, 1)]);
		let mut source = Vec::new();

		while let Ok(Some(recommendation)) = stream.next() {
			source.push(recommendation.origin.source());
		}

		assert_eq!(source, vec!["island isl0", "island isl1"]);
	}
}

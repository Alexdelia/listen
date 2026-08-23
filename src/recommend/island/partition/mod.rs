mod cluster;
mod genre;
mod name;
mod requested;

use crate::declaration::Source;

use super::seed::{Library, Seed, mean_q};

const THRESHOLD: f64 = 0.15;
const MIN_MEMBER: usize = 10;

pub(super) struct Request {
	pub recording: Vec<Source>,
	pub genre: Vec<String>,
}

impl Request {
	fn asked(&self) -> bool {
		!self.recording.is_empty() || !self.genre.is_empty()
	}
}

pub(super) struct Island {
	pub name: String,
	pub member: Vec<usize>,
}

impl Island {
	pub(super) fn q(&self, seed: &[Seed]) -> f32 {
		mean_q(self.member.iter().filter_map(|member| seed.get(*member)))
	}
}

pub(super) fn of(
	library: &Library,
	granularity: f64,
	request: &Request,
) -> hmerr::Result<Vec<Island>> {
	let genre: Vec<Vec<String>> = library
		.seed
		.iter()
		.map(|seed| genre::read(seed.mbid))
		.collect();

	if request.asked() {
		let (name, member) = requested::island(library, &genre, request)?;

		return Ok(vec![Island { name, member }]);
	}

	let similarity = cluster::similarity(&library.seed, library.user.len());
	let label = cluster::detect(&similarity, THRESHOLD, granularity, MIN_MEMBER);

	let mut member: Vec<Vec<usize>> = Vec::new();
	for (seed, label) in label.iter().enumerate() {
		if member.len() <= *label {
			member.resize_with(*label + 1, Vec::new);
		}
		member[*label].push(seed);
	}
	member.retain(|member| !member.is_empty());

	let island: Vec<Island> = name::name(&genre, &member)
		.into_iter()
		.zip(member)
		.map(|(name, member)| Island { name, member })
		.collect();

	Ok(island)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn request(recording: usize, genre: usize) -> Request {
		Request {
			recording: (0..recording)
				.map(|byte| Source::from_bytes([u8::try_from(byte).unwrap_or_default(); 16]))
				.collect(),
			genre: (0..genre).map(|token| token.to_string()).collect(),
		}
	}

	#[test]
	fn no_flag_is_no_request() {
		assert!(!request(0, 0).asked());
		assert!(request(1, 0).asked());
		assert!(request(0, 1).asked());
		assert!(request(1, 1).asked());
	}
}

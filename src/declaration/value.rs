use super::{Q, Q_MAX};

pub type Value = u8;

const VALUE: [Value; Q_MAX as usize + 1] = [
	5,   // 0.25
	50,  // 2.5
	70,  // 3.5
	90,  // 4.5
	100, // 5
];

pub const NEUTRAL: Value = 50;

pub fn from_q(q: Q) -> Value {
	VALUE[q as usize]
}

pub fn weight(q: Q) -> f32 {
	(f32::from(from_q(q)) - f32::from(NEUTRAL)) / f32::from(NEUTRAL)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_neutral_rating_neither_attracts_nor_repels() {
		assert!(weight(1).abs() < f32::EPSILON);
	}

	#[test]
	fn a_disliked_recording_repels() {
		assert!(weight(0) < 0.0);
	}

	#[test]
	fn a_liked_recording_attracts_more_the_higher_its_q() {
		for q in 2..=Q_MAX {
			assert!(weight(q) > 0.0, "q{q}");
			assert!(weight(q) > weight(q - 1), "q{q}");
		}
	}

	#[test]
	fn the_best_rating_reaches_full_attraction() {
		assert!((weight(Q_MAX) - 1.0).abs() < f32::EPSILON);
	}

	#[test]
	fn every_weight_comes_from_the_rating_table() {
		for q in 0..=Q_MAX {
			let expected = (f32::from(from_q(q)) - 50.0) / 50.0;
			assert!((weight(q) - expected).abs() < f32::EPSILON, "q{q}");
		}
	}
}

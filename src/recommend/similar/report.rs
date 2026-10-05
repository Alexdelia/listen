use ansi::abbrev::{B, D, F, Y};
use serde::Serialize;

use crate::{
	declaration::{Q, Source, value},
	format::human_readable_number,
};

use super::{
	super::labelled::Labelled,
	POOL,
	calibrate::Calibration,
	diversify::DIVERSITY,
	predict::{MIN_SUPPORT, NEUTRAL_PRIOR, Point, Scored},
	preference::ONE_PLAY,
};

#[derive(Serialize)]
pub(in crate::recommend) struct Report {
	tuning: Tuning,
	calibration: Calibration,
	rated: Vec<Rated>,
	candidate: usize,
	gate: f32,
	passing: usize,
	pool: usize,
	best_rejected: Option<Rejected>,
}

#[derive(Serialize)]
struct Tuning {
	one_play: f32,
	diversity: f32,
	min_support: u32,
	neutral_prior: f32,
	pool: usize,
	allow_known_artist: bool,
}

#[derive(Serialize)]
struct Rated {
	#[serde(flatten)]
	recording: Labelled,
	q: Q,
	raw: f32,
}

#[derive(Serialize)]
struct Rejected {
	mbid: Source,
	expected: f32,
}

pub(super) fn of(
	calibration: Calibration,
	point: &[Point],
	allow_known_artist: bool,
	judged: &[(f32, Scored)],
	passing: usize,
) -> Report {
	Report {
		tuning: Tuning {
			one_play: ONE_PLAY,
			diversity: DIVERSITY,
			min_support: MIN_SUPPORT,
			neutral_prior: NEUTRAL_PRIOR,
			pool: POOL,
			allow_known_artist,
		},
		calibration,
		rated: point
			.iter()
			.map(|point| Rated {
				recording: Labelled(point.mbid),
				q: point.q,
				raw: point.raw,
			})
			.collect(),
		candidate: judged.len(),
		gate: f32::from(value::NEUTRAL),
		passing,
		pool: passing.min(POOL),
		best_rejected: (passing == 0).then(|| judged.first()).flatten().map(
			|(expected, scored)| Rejected {
				mbid: scored.mbid,
				expected: *expected,
			},
		),
	}
}

impl Report {
	pub(in crate::recommend) fn print(&self) {
		if self.passing > 0 {
			println!(
				"{B}similar{D} {Y}{passing}{D} {F}of{D} {total} {F}candidates reach q1 ({calibration}){D}",
				passing = self.passing,
				total =
					human_readable_number::text(u64::try_from(self.candidate).unwrap_or(u64::MAX)),
				calibration = self.calibration,
			);
			return;
		}

		match &self.best_rejected {
			Some(best) => println!(
				"{Y}no candidate reaches q1{D}{F}, best{D} {expected:.0} {B}{mbid}{D}",
				expected = best.expected,
				mbid = best.mbid
			),
			None => println!("{Y}no candidate at all{D}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{super::predict::Scored, *};

	fn scored(byte: u8) -> Scored {
		Scored {
			recording_id: byte.into(),
			mbid: Source::from_bytes([byte; 16]),
			raw: 40.0,
			support: 5,
		}
	}

	#[test]
	fn a_run_where_nothing_passes_names_the_best_candidate_rejected() {
		let judged = [(45.0, scored(1)), (30.0, scored(2))];
		let report = of(
			Calibration::Uncalibrated { rated: 0 },
			&[],
			false,
			&judged,
			0,
		);

		assert_eq!(report.passing, 0);
		assert_eq!(report.candidate, 2);
		assert_eq!(
			report.best_rejected.map(|best| best.mbid),
			Some(Source::from_bytes([1; 16]))
		);
	}

	#[test]
	fn a_run_where_something_passes_rejects_nothing_and_caps_the_pool() {
		let judged = [(70.0, scored(1)), (60.0, scored(2))];
		let report = of(
			Calibration::Uncalibrated { rated: 0 },
			&[],
			false,
			&judged,
			2,
		);

		assert!(report.best_rejected.is_none());
		assert_eq!(report.pool, 2);
	}
}

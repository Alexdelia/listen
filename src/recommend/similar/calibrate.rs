use std::fmt::{self, Display};

use serde::Serialize;

use crate::declaration::{Q, value};

pub(super) const MIN_RATED: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Calibration {
	Fitted {
		intercept: f32,
		slope: f32,
		rated: usize,
	},
	Uncalibrated {
		rated: usize,
	},
}

impl Calibration {
	pub(super) fn fit(point: &[(Q, f32)]) -> Self {
		let rated = point.len();
		if rated < MIN_RATED {
			return Self::Uncalibrated { rated };
		}

		let count = real(rated);
		let mean_raw = point.iter().map(|(_, raw)| raw).sum::<f32>() / count;
		let mean_actual = point.iter().map(|(q, _)| actual(*q)).sum::<f32>() / count;

		let (covariance, variance) =
			point
				.iter()
				.fold((0.0f32, 0.0f32), |(covariance, variance), (q, raw)| {
					let spread = raw - mean_raw;
					(
						spread.mul_add(actual(*q) - mean_actual, covariance),
						spread.mul_add(spread, variance),
					)
				});

		if variance <= f32::EPSILON {
			return Self::Uncalibrated { rated };
		}

		let slope = covariance / variance;
		if slope <= 0.0 {
			return Self::Uncalibrated { rated };
		}

		Self::Fitted {
			intercept: slope.mul_add(-mean_raw, mean_actual),
			slope,
			rated,
		}
	}

	pub(super) const fn expected(self, raw: f32) -> f32 {
		match self {
			Self::Fitted {
				intercept, slope, ..
			} => slope.mul_add(raw, intercept),
			Self::Uncalibrated { .. } => raw,
		}
	}
}

impl Display for Calibration {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Fitted { slope, rated, .. } => {
				write!(f, "calibrated on {rated} picks, slope {slope:.2}")
			}
			Self::Uncalibrated { rated } => write!(f, "uncalibrated, {rated} rated picks"),
		}
	}
}

pub(super) fn actual(q: Q) -> f32 {
	f32::from(value::from_q(q))
}

#[expect(
	clippy::cast_precision_loss,
	reason = "a count of rated picks, orders of magnitude below the f32 mantissa"
)]
pub(super) const fn real(count: usize) -> f32 {
	count as f32
}

#[cfg(test)]
mod tests {
	use super::*;

	fn on_line(count: usize, slope: f32, intercept: f32) -> Vec<(Q, f32)> {
		(0..count)
			.map(|step| {
				let q = Q::try_from(step % 5).unwrap_or_default();
				(q, (actual(q) - intercept) / slope)
			})
			.collect()
	}

	#[test]
	fn rated_picks_on_a_line_give_that_line_back() {
		let Calibration::Fitted {
			intercept,
			slope,
			rated,
		} = Calibration::fit(&on_line(MIN_RATED, 2.0, -60.0))
		else {
			panic!("not fitted");
		};

		assert!((slope - 2.0).abs() < 1e-3, "{slope}");
		assert!((intercept + 60.0).abs() < 1e-2, "{intercept}");
		assert_eq!(rated, MIN_RATED);
	}

	#[test]
	fn too_few_rated_picks_leave_it_uncalibrated() {
		assert_eq!(
			Calibration::fit(&on_line(MIN_RATED - 1, 2.0, -60.0)),
			Calibration::Uncalibrated {
				rated: MIN_RATED - 1
			}
		);
	}

	#[test]
	fn no_rated_pick_at_all_leaves_it_uncalibrated() {
		assert_eq!(
			Calibration::fit(&[]),
			Calibration::Uncalibrated { rated: 0 }
		);
	}

	#[test]
	fn a_fit_that_falls_with_raw_is_not_trusted() {
		assert!(matches!(
			Calibration::fit(&on_line(MIN_RATED, -2.0, 160.0)),
			Calibration::Uncalibrated { .. }
		));
	}

	#[test]
	fn rated_picks_that_all_share_one_raw_cannot_be_fitted() {
		let point: Vec<(Q, f32)> = (0..MIN_RATED)
			.map(|step| (Q::try_from(step % 5).unwrap_or_default(), 55.0))
			.collect();

		assert!(matches!(
			Calibration::fit(&point),
			Calibration::Uncalibrated { .. }
		));
	}

	#[test]
	fn uncalibrated_expects_the_raw_rating() {
		let raw = 57.5;

		assert!((Calibration::Uncalibrated { rated: 0 }.expected(raw) - raw).abs() < f32::EPSILON);
	}

	#[test]
	fn the_calibration_says_what_it_rests_on() {
		let fitted = Calibration::Fitted {
			intercept: -60.0,
			slope: 2.4,
			rated: 156,
		}
		.to_string();

		assert!(fitted.contains("156"), "{fitted}");
		assert!(fitted.contains("2.40"), "{fitted}");
		assert!(
			Calibration::Uncalibrated { rated: 12 }
				.to_string()
				.contains("uncalibrated")
		);
	}
}

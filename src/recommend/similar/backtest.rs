use ansi::abbrev::{B, D, F, Y};

use crate::declaration::Q;

use super::calibrate::{Calibration, actual, real};

const BAND: usize = 5;

struct Band {
	count: usize,
	raw: f32,
	actual: f32,
	q0: f32,
	liked: f32,
}

pub(super) fn print(point: &[(Q, f32)], calibration: Calibration) {
	println!(
		"{B}{rated}{D} {F}rated picks{D}  {F}spearman{D} {Y}{rho:.2}{D}  {calibration}",
		rated = point.len(),
		rho = spearman(point),
	);
	println!("{F}band    n  raw  expected  actual   q0  q2+{D}");

	for (number, band) in band(point).iter().enumerate() {
		println!(
			"{number:>4}  {count:>3}  {raw:>3.0}  {expected:>8.0}  {actual:>6.0}  {q0:>3.0}% {liked:>3.0}%",
			number = number + 1,
			count = band.count,
			raw = band.raw,
			expected = calibration.expected(band.raw),
			actual = band.actual,
			q0 = band.q0 * 100.0,
			liked = band.liked * 100.0,
		);
	}
}

fn spearman(point: &[(Q, f32)]) -> f32 {
	if point.len() < 2 {
		return 0.0;
	}

	let q: Vec<f32> = point.iter().map(|(q, _)| f32::from(*q)).collect();
	let raw: Vec<f32> = point.iter().map(|(_, raw)| *raw).collect();

	pearson(&rank(&q), &rank(&raw))
}

fn rank(value: &[f32]) -> Vec<f32> {
	let mut order: Vec<usize> = (0..value.len()).collect();
	order.sort_by(|a, b| value[*a].total_cmp(&value[*b]));

	let mut rank = vec![0.0; value.len()];
	let mut start = 0;

	while start < order.len() {
		let mut end = start;
		while end + 1 < order.len()
			&& value[order[end + 1]]
				.total_cmp(&value[order[start]])
				.is_eq()
		{
			end += 1;
		}

		let shared = real(start + end) / 2.0 + 1.0;
		for position in start..=end {
			rank[order[position]] = shared;
		}

		start = end + 1;
	}

	rank
}

fn pearson(x: &[f32], y: &[f32]) -> f32 {
	let count = real(x.len());
	let mean_x = x.iter().sum::<f32>() / count;
	let mean_y = y.iter().sum::<f32>() / count;

	let (covariance, variance_x, variance_y) = x.iter().zip(y).fold(
		(0.0f32, 0.0f32, 0.0f32),
		|(covariance, variance_x, variance_y), (x, y)| {
			let (dx, dy) = (x - mean_x, y - mean_y);
			(
				dx.mul_add(dy, covariance),
				dx.mul_add(dx, variance_x),
				dy.mul_add(dy, variance_y),
			)
		},
	);

	let norm = (variance_x * variance_y).sqrt();
	if norm <= f32::EPSILON {
		return 0.0;
	}

	covariance / norm
}

fn band(point: &[(Q, f32)]) -> Vec<Band> {
	let mut sorted = point.to_vec();
	sorted.sort_by(|a, b| a.1.total_cmp(&b.1));

	let size = sorted.len().div_ceil(BAND).max(1);

	sorted
		.chunks(size)
		.map(|chunk| {
			let count = real(chunk.len());
			Band {
				count: chunk.len(),
				raw: chunk.iter().map(|(_, raw)| raw).sum::<f32>() / count,
				actual: chunk.iter().map(|(q, _)| actual(*q)).sum::<f32>() / count,
				q0: real(chunk.iter().filter(|(q, _)| *q == 0).count()) / count,
				liked: real(chunk.iter().filter(|(q, _)| *q >= 2).count()) / count,
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_raw_rating_ordered_like_the_q_correlates_fully() {
		let point = [(0, 10.0), (1, 20.0), (2, 30.0), (3, 40.0), (4, 50.0)];

		assert!((spearman(&point) - 1.0).abs() < 1e-6);
	}

	#[test]
	fn a_raw_rating_ordered_against_the_q_correlates_negatively() {
		let point = [(0, 50.0), (1, 40.0), (2, 30.0), (3, 20.0), (4, 10.0)];

		assert!((spearman(&point) + 1.0).abs() < 1e-6);
	}

	#[test]
	fn tied_q_share_their_rank() {
		let point = [(0, 10.0), (0, 20.0), (4, 30.0), (4, 40.0)];

		assert!(spearman(&point) > 0.85);
	}

	#[test]
	fn too_few_points_correlate_to_nothing() {
		assert!(spearman(&[(0, 10.0)]).abs() < f32::EPSILON);
		assert!(spearman(&[]).abs() < f32::EPSILON);
	}

	#[test]
	fn rated_picks_split_into_bands_by_raw_rating() {
		let point: Vec<(Q, f32)> = (0..10u8).map(|step| (step % 5, f32::from(step))).collect();

		let band = band(&point);

		assert_eq!(band.len(), BAND);
		assert!(band.windows(2).all(|pair| pair[0].raw < pair[1].raw));
		assert_eq!(band.iter().map(|band| band.count).sum::<usize>(), 10);
	}

	#[test]
	fn a_band_counts_its_share_of_q0_and_of_q2_and_above() {
		let band = band(&[(0, 1.0), (0, 2.0), (2, 3.0), (4, 4.0), (1, 5.0)]);
		let all: Vec<&Band> = band.iter().collect();

		assert_eq!(all.len(), 5);
		assert!((all[0].q0 - 1.0).abs() < 1e-6);
		assert!((all[2].liked - 1.0).abs() < 1e-6);
	}
}

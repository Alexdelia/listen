use ansi::{abbrev::D, hex};

use crate::declaration::Q;

pub(super) const Q0: &str = hex!(#d1ba47);
pub(super) const Q1: &str = hex!(#a1d147);
pub(super) const Q2: &str = hex!(#47d160);
pub(super) const Q3: &str = hex!(#8147d1);
pub(super) const Q4: &str = hex!(#fc0380);

pub(crate) const fn q_color(q: Q) -> &'static str {
	match q {
		0 => Q0,
		1 => Q1,
		2 => Q2,
		3 => Q3,
		4 => Q4,
		_ => D,
	}
}

#[expect(
	clippy::cast_possible_truncation,
	clippy::cast_sign_loss,
	reason = "an average q, always within the q range"
)]
pub(crate) const fn q_f32_color(q: f32) -> &'static str {
	q_color(q.floor() as Q)
}

use console::Style;

use crate::declaration::Q;

const fn hex(rgb: u32) -> Style {
	let [_, r, g, b] = rgb.to_be_bytes();
	Style::new().true_color(r, g, b)
}

pub(super) const Q0: Style = hex(0xd1_ba_47);
pub(super) const Q1: Style = hex(0xa1_d1_47);
pub(super) const Q2: Style = hex(0x47_d1_60);
pub(super) const Q3: Style = hex(0x81_47_d1);
pub(super) const Q4: Style = hex(0xfc_03_80);

pub(crate) const fn q_style(q: Q) -> Style {
	match q {
		0 => Q0,
		1 => Q1,
		2 => Q2,
		3 => Q3,
		4 => Q4,
		_ => Style::new(),
	}
}

#[expect(
	clippy::cast_possible_truncation,
	clippy::cast_sign_loss,
	reason = "an average q, always within the q range"
)]
pub(crate) const fn q_f32_style(q: f32) -> Style {
	q_style(q.floor() as Q)
}

#[cfg(test)]
mod tests {
	use console::Style;

	use super::*;

	#[test]
	fn a_q_past_the_top_is_unstyled() {
		assert_eq!(q_style(5), Style::new());
		assert_eq!(q_f32_style(7.9), Style::new());
	}

	#[test]
	fn each_q_keeps_its_colour() {
		assert_eq!(q_style(4), Style::new().true_color(0xfc, 0x03, 0x80));
		assert_eq!(q_f32_style(0.5), Style::new().true_color(0xd1, 0xba, 0x47));
	}
}

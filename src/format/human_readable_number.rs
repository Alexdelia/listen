use ansi::abbrev::{D, F};

const GROUP: usize = 3;
const SEPARATOR: char = ' ';
const SCALE: [(u64, &str); 5] = [
	(1_000_000_000_000_000_000, "E"),
	(1_000_000_000_000_000, "P"),
	(1_000_000_000_000, "T"),
	(1_000_000_000, "B"),
	(1_000_000, "M"),
];

pub(crate) fn split(n: u64) -> (String, &'static str) {
	for (scale, unit) in SCALE {
		if n >= scale {
			return (scaled(n, scale), unit);
		}
	}

	(grouped(n), "")
}

pub(crate) fn text(n: u64) -> String {
	let (value, unit) = split(n);
	if unit.is_empty() {
		return value;
	}

	format!("{value}{F}{unit}{D}")
}

fn scaled(n: u64, scale: u64) -> String {
	let tenth = n / (scale / 10);
	let whole = tenth / 10;
	let frac = tenth % 10;

	if frac == 0 {
		return whole.to_string();
	}

	format!("{whole}.{frac}")
}

fn grouped(n: u64) -> String {
	let digit = n.to_string();
	let mut out = String::with_capacity(digit.len() + digit.len() / GROUP);

	for (i, c) in digit.char_indices() {
		if i > 0 && (digit.len() - i).is_multiple_of(GROUP) {
			out.push(SEPARATOR);
		}
		out.push(c);
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_small_number_is_left_alone() {
		assert_eq!(split(0), ("0".to_string(), ""));
		assert_eq!(split(7), ("7".to_string(), ""));
		assert_eq!(split(999), ("999".to_string(), ""));
	}

	#[test]
	fn digit_below_a_million_are_grouped_by_three() {
		assert_eq!(split(1_000), ("1 000".to_string(), ""));
		assert_eq!(split(26_180), ("26 180".to_string(), ""));
		assert_eq!(split(100_000), ("100 000".to_string(), ""));
		assert_eq!(split(999_999), ("999 999".to_string(), ""));
	}

	#[test]
	fn a_round_scaled_number_drops_its_fraction() {
		assert_eq!(split(1_000_000), ("1".to_string(), "M"));
		assert_eq!(split(2_000_000_000), ("2".to_string(), "B"));
		assert_eq!(split(5_000_000_000_000), ("5".to_string(), "T"));
	}

	#[test]
	fn a_scaled_number_keeps_one_truncated_decimal() {
		assert_eq!(split(10_733_828), ("10.7".to_string(), "M"));
		assert_eq!(split(336_699_506), ("336.6".to_string(), "M"));
		assert_eq!(split(1_290_000_000), ("1.2".to_string(), "B"));
	}

	#[test]
	fn truncating_never_carries_into_the_next_unit() {
		assert_eq!(split(999_999_999), ("999.9".to_string(), "M"));
	}

	#[test]
	fn the_unit_is_faint_and_the_grouped_number_has_none() {
		assert_eq!(text(1_000_000), format!("1{F}M{D}"));
		assert_eq!(text(1_000), "1 000".to_string());
	}
}

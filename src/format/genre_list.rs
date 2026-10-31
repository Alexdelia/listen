use console::style;

pub(crate) const SEPARATOR: &str = " / ";

pub(crate) fn text(list: &str) -> String {
	list.split(SEPARATOR)
		.map(|genre| style(&genre).dim().white().to_string())
		.collect::<Vec<_>>()
		.join(&style(&SEPARATOR).dim().to_string())
}

pub(crate) fn width(list: &str) -> usize {
	list.chars().count()
}

pub(crate) fn pad(list: &str, width: usize) -> String {
	" ".repeat(width.saturating_sub(self::width(list)))
}

#[cfg(test)]
mod tests {
	use console::strip_ansi_codes;

	use super::*;

	#[test]
	fn a_single_genre_reads_as_itself() {
		assert_eq!(strip_ansi_codes(&text("eurobeat")), "eurobeat");
	}

	#[test]
	fn the_genres_and_their_separator_read_in_order() {
		assert_eq!(
			strip_ansi_codes(&text("eurobeat / touhou")),
			"eurobeat / touhou"
		);
	}

	#[test]
	fn a_slash_inside_a_genre_is_kept_as_written() {
		assert_eq!(strip_ansi_codes(&text("meter:4/4")), "meter:4/4");
	}

	#[test]
	fn the_width_ignores_the_escape_the_color_adds() {
		assert_eq!(width("eurobeat / touhou"), 17);
	}

	#[test]
	fn padding_fills_up_to_the_widest_list() {
		assert_eq!(pad("touhou", 10), "    ");
		assert_eq!(pad("touhou", 6), "");
		assert_eq!(pad("touhou", 2), "");
	}
}

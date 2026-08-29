use ansi::{
	WHITE,
	abbrev::{D, F},
};

pub(crate) const SEPARATOR: &str = " / ";

pub(crate) fn text(list: &str) -> String {
	list.split(SEPARATOR)
		.map(|genre| format!("{F}{WHITE}{genre}{D}"))
		.collect::<Vec<_>>()
		.join(&format!("{F}{SEPARATOR}{D}"))
}

pub(crate) fn width(list: &str) -> usize {
	list.chars().count()
}

pub(crate) fn pad(list: &str, width: usize) -> String {
	" ".repeat(width.saturating_sub(self::width(list)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_single_genre_carries_the_genre_color() {
		assert_eq!(text("eurobeat"), format!("{F}{WHITE}eurobeat{D}"));
	}

	#[test]
	fn the_separator_is_faint_and_uncolored() {
		assert_eq!(
			text("eurobeat / touhou"),
			format!("{F}{WHITE}eurobeat{D}{F}{SEPARATOR}{D}{F}{WHITE}touhou{D}")
		);
	}

	#[test]
	fn a_slash_inside_a_genre_is_not_a_separator() {
		assert_eq!(text("meter:4/4"), format!("{F}{WHITE}meter:4/4{D}"));
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

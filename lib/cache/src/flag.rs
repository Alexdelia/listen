const FALSE: [&str; 4] = ["0", "false", "no", "off"];

#[must_use]
pub fn enabled(key: &str) -> bool {
	enabled_or(key, false)
}

#[must_use]
pub fn enabled_or(key: &str, default: bool) -> bool {
	enabled_or_by(std::env::var(key).ok().as_deref(), default)
}

fn enabled_or_by(value: Option<&str>, default: bool) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map_or(default, |value| {
			!FALSE.contains(&value.to_lowercase().as_str())
		})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn enabled_by(value: Option<&str>) -> bool {
		enabled_or_by(value, false)
	}

	#[test]
	fn an_unset_or_blank_flag_is_off() {
		assert!(!enabled_by(None));
		assert!(!enabled_by(Some("")));
		assert!(!enabled_by(Some("  ")));
	}

	#[test]
	fn every_false_spelling_is_off_whatever_its_case_or_padding() {
		for value in ["0", "false", "no", "off", "FALSE", " Off ", "No"] {
			assert!(!enabled_by(Some(value)), "{value:?}");
		}
	}

	#[test]
	fn an_unset_or_blank_default_on_flag_is_on() {
		assert!(enabled_or_by(None, true));
		assert!(enabled_or_by(Some("  "), true));
	}

	#[test]
	fn a_default_on_flag_turns_off_on_any_false_spelling() {
		for value in ["0", "false", "no", "off", " Off "] {
			assert!(!enabled_or_by(Some(value), true), "{value:?}");
		}
	}

	#[test]
	fn anything_else_turns_the_flag_on() {
		for value in ["true", "1", "yes", "TRUE", " on "] {
			assert!(enabled_by(Some(value)), "{value:?}");
		}
	}
}

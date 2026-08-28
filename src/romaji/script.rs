pub(crate) fn latin(text: &str) -> bool {
	!text.chars().any(|c| c.is_alphabetic() && !is_latin(c))
}

pub(crate) fn kana(text: &str) -> bool {
	let mut seen = false;

	for c in text.chars() {
		if !c.is_alphabetic() {
			continue;
		}

		if is_kana(c) {
			seen = true;
		} else if !is_latin(c) {
			return false;
		}
	}

	seen
}

const fn is_latin(c: char) -> bool {
	c.is_ascii_alphabetic() || matches!(c, '\u{00c0}'..='\u{024f}' | '\u{1e00}'..='\u{1eff}')
}

const fn is_kana(c: char) -> bool {
	matches!(c, '\u{3041}'..='\u{309f}' | '\u{30a0}'..='\u{30ff}')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_title_of_ascii_letters_is_latin() {
		assert!(latin("Secret base"));
	}

	#[test]
	fn a_macron_and_a_hyphen_that_is_not_ascii_stay_latin() {
		assert!(latin("Gunjō"));
		assert!(latin("K‐391"));
	}

	#[test]
	fn kanji_hangul_and_cyrillic_are_not_latin() {
		assert!(!latin("ひみつ基地"));
		assert!(!latin("우린 좀 달라"));
		assert!(!latin("Кончится лето"));
	}

	#[test]
	fn a_title_written_only_in_kana_is_kana() {
		assert!(kana("ひみつきち"));
		assert!(kana("インフェルノ"));
	}

	#[test]
	fn kana_next_to_latin_is_still_kana() {
		assert!(kana("バイバイ YESTERDAY"));
	}

	#[test]
	fn a_prolonged_sound_mark_does_not_leave_kana() {
		assert!(kana("ミスターフィクサー"));
	}

	#[test]
	fn kanji_among_kana_is_not_kana() {
		assert!(!kana("ひみつ基地"));
	}

	#[test]
	fn latin_alone_is_not_kana() {
		assert!(!kana("Secret base"));
	}

	#[test]
	fn cyrillic_is_neither_latin_nor_kana() {
		assert!(!kana("Кончится лето"));
	}
}

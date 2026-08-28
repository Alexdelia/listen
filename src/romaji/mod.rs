mod script;

use wana_kana::ConvertJapanese;

pub(crate) use script::{kana, latin};

pub(crate) fn of(text: &str) -> Option<String> {
	script::kana(text).then(|| text.to_romaji())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_kana_title_has_a_romaji_version() {
		assert_eq!(of("ひみつきち").as_deref(), Some("himitsukichi"));
		assert_eq!(
			of("あおいはるとにしのそら").as_deref(),
			Some("aoiharutonishinosora")
		);
	}

	#[test]
	fn katakana_reads_the_same_as_hiragana() {
		assert_eq!(of("アブラカダブラ").as_deref(), Some("aburakadabura"));
	}

	#[test]
	fn a_small_tsu_doubles_the_consonant_after_it() {
		assert_eq!(
			of("せいしゅんコンプレックス").as_deref(),
			Some("seishunkonpurekkusu")
		);
	}

	#[test]
	fn a_syllabic_n_before_a_vowel_takes_an_apostrophe() {
		assert_eq!(of("しんや").as_deref(), Some("shin'ya"));
	}

	#[test]
	fn latin_inside_a_kana_title_is_left_alone() {
		assert_eq!(
			of("バイバイ YESTERDAY").as_deref(),
			Some("baibai YESTERDAY")
		);
	}

	#[test]
	fn kanji_has_no_romaji_version_of_its_own() {
		assert_eq!(of("ひみつ基地"), None);
	}

	#[test]
	fn what_is_already_latin_does_not_move() {
		assert_eq!(of("Secret base"), None);
	}

	#[test]
	fn a_script_romaji_does_not_serve_does_not_move() {
		assert_eq!(of("Кончится лето"), None);
		assert_eq!(of("우린 좀 달라"), None);
	}
}

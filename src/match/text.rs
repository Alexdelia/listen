use wana_kana::{ConvertJapanese, IsJapaneseStr};

pub(super) fn normalize_title(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	let mut depth: u32 = 0;

	for c in s.chars() {
		match c {
			'(' | '[' | '{' | '<' => depth += 1,
			')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
			_ if depth == 0 => out.push(c),
			_ => {}
		}
	}

	normalize(&out)
}

fn normalize(s: &str) -> String {
	s.chars()
		.flat_map(char::to_lowercase)
		.filter(|c| c.is_alphanumeric())
		.collect()
}

pub(super) fn romanize(s: &str) -> Option<String> {
	if s.is_empty() || s.contains_kanji() {
		return None;
	}

	let romaji = s.to_romaji();
	if normalize(&romaji) == normalize(s) {
		return None;
	}

	Some(romaji)
}

pub(super) fn is_latin(s: &str) -> bool {
	!s.is_empty() && s.is_romaji()
}

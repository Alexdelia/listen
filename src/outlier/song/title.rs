use super::word::word;

const BRACKET_OPEN: [char; 7] = ['(', '[', '{', '<', '（', '【', '「'];
const BRACKET_CLOSE: [char; 7] = [')', ']', '}', '>', '）', '】', '」'];

const VERSION_MARKER: [&str; 16] = [
	"remix",
	"mix",
	"edit",
	"version",
	"ver",
	"live",
	"cover",
	"instrumental",
	"inst",
	"acoustic",
	"remaster",
	"remastered",
	"vip",
	"extended",
	"official",
	"video",
];

const VERSION_MARKER_FRAGMENT: [&str; 7] = [
	"インスト",
	"リミックス",
	"カバー",
	"アレンジ",
	"バージョン",
	"ライブ",
	"ライヴ",
];

pub(super) fn stripped(title: &str) -> String {
	let mut word = word(&remove_edge_bracket(title));

	while word.last().is_some_and(|w| is_version_marker(w)) {
		word.pop();
	}

	word.join(" ")
}

fn remove_edge_bracket(s: &str) -> String {
	let mut c: Vec<char> = s.trim().chars().collect();

	loop {
		if c.last().is_some_and(|l| BRACKET_CLOSE.contains(l)) {
			let Some(open) = matching_open(&c) else { break };
			c.truncate(open);
		} else if c.first().is_some_and(|f| BRACKET_OPEN.contains(f)) {
			let Some(close) = matching_close(&c) else {
				break;
			};
			c.drain(..=close);
		} else {
			break;
		}

		while c.last().is_some_and(|l| l.is_whitespace()) {
			c.pop();
		}
		while c.first().is_some_and(|f| f.is_whitespace()) {
			c.remove(0);
		}
	}

	c.into_iter().collect()
}

fn matching_open(c: &[char]) -> Option<usize> {
	balanced(c.iter().enumerate().rev(), &BRACKET_CLOSE, &BRACKET_OPEN)
}

fn matching_close(c: &[char]) -> Option<usize> {
	balanced(c.iter().enumerate(), &BRACKET_OPEN, &BRACKET_CLOSE)
}

fn balanced<'c>(
	char: impl Iterator<Item = (usize, &'c char)>,
	deeper: &[char],
	shallower: &[char],
) -> Option<usize> {
	let mut depth = 0usize;

	for (i, ch) in char {
		if deeper.contains(ch) {
			depth += 1;
		} else if shallower.contains(ch) {
			depth -= 1;
			if depth == 0 {
				return Some(i);
			}
		}
	}

	None
}

fn is_version_marker(word: &str) -> bool {
	VERSION_MARKER.contains(&word) || VERSION_MARKER_FRAGMENT.iter().any(|m| word.contains(m))
}

const ARTIST_SEPARATOR: [&str; 14] = [
	"&", "/", ";", ",", "+", ":", "、", "×", " feat. ", " feat ", " ft. ", " ft ", " x ", " with ",
];

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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(super) enum Match {
	Stripped,
	Exact,
}

pub(super) struct Song {
	title: String,
	stripped: String,
	artist: Vec<Artist>,
}

impl Song {
	pub(super) fn new(title: &str, artist: &str) -> Self {
		Self {
			title: normalize(title),
			stripped: stripped(title),
			artist: artists(artist),
		}
	}

	pub(super) fn matches(&self, other: &Self) -> Option<Match> {
		if self.title.is_empty() {
			return None;
		}

		let title = if self.title == other.title {
			Match::Exact
		} else if !self.stripped.is_empty() && self.stripped == other.stripped {
			Match::Stripped
		} else {
			return None;
		};

		compatible(&self.artist, &other.artist, title).then_some(title)
	}

	pub(super) fn same_title(&self, other: &Self) -> bool {
		!self.title.is_empty() && self.title == other.title
	}

	pub(super) fn same_stripped_title(&self, other: &Self) -> bool {
		!self.stripped.is_empty() && self.stripped == other.stripped
	}
}

struct Artist {
	concat: String,
	latin: bool,
	other: bool,
}

impl Artist {
	fn new(name: &str) -> Self {
		Self {
			concat: name.replace(' ', ""),
			latin: name.chars().any(|c| c.is_ascii_alphabetic()),
			other: name.chars().any(|c| c.is_alphabetic() && !c.is_ascii()),
		}
	}

	fn same(&self, other: &Self) -> bool {
		self.concat == other.concat
	}

	fn script_comparable(&self, other: &Self) -> bool {
		(self.latin && other.latin) || (self.other && other.other)
	}
}

fn compatible(a: &[Artist], b: &[Artist], title: Match) -> bool {
	if a.is_empty() || b.is_empty() {
		return true;
	}

	if a.iter().any(|x| b.iter().any(|y| x.same(y))) {
		return true;
	}

	title == Match::Exact && !a.iter().any(|x| b.iter().any(|y| x.script_comparable(y)))
}

fn normalize(s: &str) -> String {
	word(s).join(" ")
}

fn word(s: &str) -> Vec<String> {
	s.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|w| !w.is_empty())
		.map(str::to_string)
		.collect()
}

fn stripped(title: &str) -> String {
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
	let mut depth = 0usize;

	for (i, ch) in c.iter().enumerate().rev() {
		if BRACKET_CLOSE.contains(ch) {
			depth += 1;
		} else if BRACKET_OPEN.contains(ch) {
			depth -= 1;
			if depth == 0 {
				return Some(i);
			}
		}
	}

	None
}

fn matching_close(c: &[char]) -> Option<usize> {
	let mut depth = 0usize;

	for (i, ch) in c.iter().enumerate() {
		if BRACKET_OPEN.contains(ch) {
			depth += 1;
		} else if BRACKET_CLOSE.contains(ch) {
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

fn artists(artist: &str) -> Vec<Artist> {
	let mut split = artist.to_lowercase();
	for separator in ARTIST_SEPARATOR {
		split = split.replace(separator, "\n");
	}

	split
		.split('\n')
		.map(normalize)
		.filter(|a| !a.is_empty())
		.map(|name| Artist::new(&name))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn join_phrase_still_matches() {
		let a = Song::new("Somebody That I Used to Know", "Gotye & Kimbra");
		let b = Song::new("Somebody That I Used to Know", "Gotye feat. Kimbra");

		assert_eq!(a.matches(&b), Some(Match::Exact));
	}

	#[test]
	fn punctuation_and_case_ignored() {
		let a = Song::new("Do I Wanna Know?", "Arctic Monkeys");
		let b = Song::new("do i wanna know", "arctic monkeys");

		assert_eq!(a.matches(&b), Some(Match::Exact));
	}

	#[test]
	fn different_title_does_not_match() {
		let a = Song::new("The Wolf", "SIAMES");
		let b = Song::new("The Fox", "SIAMES");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn different_artist_does_not_match() {
		let a = Song::new("Hurt", "Nine Inch Nails");
		let b = Song::new("Hurt", "Johnny Cash");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn empty_title_never_matches() {
		let a = Song::new("", "Arctic Monkeys");
		let b = Song::new("", "Arctic Monkeys");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn bracket_suffix_matches() {
		let a = Song::new("Nameless World (official video)", "Skip the Use");
		let b = Song::new("Nameless World", "Skip the Use");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
		assert_eq!(b.matches(&a), Some(Match::Stripped));
	}

	#[test]
	fn live_bracket_matches() {
		let a = Song::new("Angreifer", "Unlucky Morpheus");
		let b = Song::new(
			"Angreifer (LIVE 2022 at Zepp DiverCity)",
			"Unlucky Morpheus",
		);

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn bracket_variant_matches_on_both_side() {
		let a = Song::new("Distortion World (Pokémon Platinum Arrangement)", "mewmore");
		let b = Song::new("Distortion World (Pokémon Platinum Remix)", "mewmore");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn version_suffix_word_matches() {
		let a = Song::new(
			"Everything will freeze ギターインスト編",
			"UNDEAD CORPORATION",
		);
		let b = Song::new("Everything Will Freeze", "UNDEAD CORPORATION");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn vip_edit_matches() {
		let a = Song::new("Vitality (VIP edit)", "Mittsies");
		let b = Song::new("Vitality", "Mittsies");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn remix_bracket_matches_original() {
		let a = Song::new("Empire of Steel (MASKED remix)", "Essenger feat. Scandroid");
		let b = Song::new("Empire of Steel", "Essenger & Scandroid");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn spaceless_artist_matches() {
		let a = Song::new(
			"Dear You (Higurashi Vocal Drum and Bass Remix)",
			"DJGenericname",
		);
		let b = Song::new("dear you", "dj genericname");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn transliterated_artist_is_not_compared_on_exact_title() {
		let a = Song::new("Stay Alive", "エミリア & 高橋李依");
		let b = Song::new("Stay Alive", "Emilia");

		assert_eq!(a.matches(&b), Some(Match::Exact));
	}

	#[test]
	fn transliterated_artist_does_not_reach_through_stripped_title() {
		let a = Song::new("Bad Apple!! （Tetsuya Komuro Remix）", "小室哲哉 & のみこ");
		let b = Song::new("Bad Apple!!", "Mini Miku");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn artist_separator_variants_match() {
		let comma = Song::new("State Lines", "Hybrid Minds, Birdy");
		let ampersand = Song::new("State Lines", "Hybrid Minds & Birdy");
		assert_eq!(comma.matches(&ampersand), Some(Match::Exact));

		let with = Song::new("First Time", "Kygo with Ellie Goulding");
		let and = Song::new("First Time", "Kygo & Ellie Goulding");
		assert_eq!(with.matches(&and), Some(Match::Exact));

		let plus = Song::new("DARK SUN", "TOKYO ROSE + ALEX");
		let both = Song::new("DARK SUN", "TOKYO ROSE & ALEX");
		assert_eq!(plus.matches(&both), Some(Match::Exact));
	}

	#[test]
	fn mid_title_bracket_is_part_of_the_title() {
		let a = Song::new("I (Can't) See You", "Wisp X");
		let b = Song::new("I See You", "MISSIO");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn leading_and_trailing_bracket_are_stripped() {
		let a = Song::new(
			"[C94] Instrumental Collection Vol.1 [Album XFD]",
			"Norowareta Night",
		);
		let b = Song::new("Instrumental Collection Vol.1", "Norowareta Night");

		assert_eq!(a.matches(&b), Some(Match::Stripped));
	}

	#[test]
	fn same_title_different_cover_artist_does_not_match() {
		let a = Song::new("Bad Apple!!", "Alstroemeria Records");
		let b = Song::new("Bad Apple!!", "RichaadEB & Cristina Vee");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn same_piece_different_performer_does_not_match() {
		let a = Song::new("Gnossienne no. 1", "Pascal Rogé");
		let b = Song::new("Gnossienne no. 1", "Otto Tolonen");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn sequel_does_not_match() {
		let a = Song::new("Dreams Pt. II", "Lost Sky feat. Sara Skinner");
		let b = Song::new("Dreams", "Lost Sky");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn same_bracket_different_title_does_not_match() {
		let a = Song::new("Für Elise (Epic Trailer version)", "Hidden Citizens");
		let b = Song::new("Moonlight Sonata (Epic Trailer version)", "Hidden Citizens");

		assert_eq!(a.matches(&b), None);
	}

	#[test]
	fn distinct_version_suffix_stays_distinct() {
		let a = Song::new("Hacking to the Gate", "いとうかなこ");
		let b = Song::new("Hacking to the Gate -symphonic ver.-", "いとうかなこ");

		assert_eq!(a.matches(&b), None);
	}
}

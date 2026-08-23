use super::{Match, word::normalize};

const ARTIST_SEPARATOR: [&str; 14] = [
	"&", "/", ";", ",", "+", ":", "、", "×", " feat. ", " feat ", " ft. ", " ft ", " x ", " with ",
];

pub(super) struct Artist {
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

	const fn script_comparable(&self, other: &Self) -> bool {
		(self.latin && other.latin) || (self.other && other.other)
	}
}

pub(super) fn artists(artist: &str) -> Vec<Artist> {
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

pub(super) fn compatible(a: &[Artist], b: &[Artist], title: Match) -> bool {
	if a.is_empty() || b.is_empty() {
		return true;
	}

	if a.iter().any(|x| b.iter().any(|y| x.same(y))) {
		return true;
	}

	title == Match::Exact && !a.iter().any(|x| b.iter().any(|y| x.script_comparable(y)))
}

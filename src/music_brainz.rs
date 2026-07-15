use const_format::concatcp;

const AUTHOR: &str = author_name(env!("CARGO_PKG_AUTHORS"));

pub const CLIENT: &str = concatcp!(
	AUTHOR,
	"/",
	env!("CARGO_PKG_NAME"),
	"-",
	env!("CARGO_PKG_VERSION"),
);

pub const USER_AGENT: &str = concatcp!(CLIENT, " ( https://github.com/Alexdelia/listen )");

const fn author_name(authors: &str) -> &str {
	let bytes = authors.as_bytes();
	let mut end = 0;
	while end < bytes.len() && bytes[end] != b' ' {
		end += 1;
	}
	authors.split_at(end).0
}

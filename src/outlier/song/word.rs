pub(super) fn normalize(s: &str) -> String {
	word(s).join(" ")
}

pub(super) fn word(s: &str) -> Vec<String> {
	s.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|w| !w.is_empty())
		.map(str::to_string)
		.collect()
}

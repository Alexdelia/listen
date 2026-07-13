use super::push_unique;
use super::text::is_latin;

const MAX_QUERY: usize = 5;

pub(super) fn build(title_form: &[String], artist: &[Vec<String>]) -> Vec<String> {
	let mut out = Vec::new();

	let latin_artist = join(artist, latin_form);
	let raw_artist = join(artist, |form| form.first().cloned());

	if let Some(latin_title) = title_form
		.iter()
		.find(|f| is_latin(f))
		.or(title_form.first())
	{
		push_unique(&mut out, query(&latin_artist, latin_title));
	}

	for title in title_form {
		push_unique(&mut out, query(&raw_artist, title));
		push_unique(&mut out, query(&latin_artist, title));
		if is_latin(title) {
			push_unique(&mut out, title.clone());
		}
	}

	out.truncate(MAX_QUERY);
	out
}

fn join(artist: &[Vec<String>], pick: impl Fn(&[String]) -> Option<String>) -> String {
	artist
		.iter()
		.filter_map(|form| pick(form))
		.collect::<Vec<_>>()
		.join(" ")
}

fn latin_form(form: &[String]) -> Option<String> {
	form.iter()
		.find(|f| is_latin(f))
		.or_else(|| form.first())
		.cloned()
}

fn query(artist: &str, title: &str) -> String {
	format!("{artist} {title}").trim().to_string()
}

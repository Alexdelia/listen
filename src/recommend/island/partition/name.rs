use std::collections::HashMap;

use crate::library::tag::GENRE_SEPARATOR;

use super::super::real;

const TOKEN_IN_NAME: usize = 3;
const MIN_TOKEN_OCCURRENCE: usize = 2;
const MIN_TOKEN_SHARE: f32 = 0.05;

pub(super) fn name(genre: &[Vec<String>], member: &[Vec<usize>]) -> Vec<String> {
	let (library, tagged) = tally(genre.iter());

	let name = member
		.iter()
		.enumerate()
		.map(|(island, member)| {
			over_represented(member, genre, &library, tagged)
				.unwrap_or_else(|| format!("island {island}"))
		})
		.collect();

	distinct(name)
}

fn over_represented(
	member: &[usize],
	genre: &[Vec<String>],
	library: &HashMap<&str, usize>,
	tagged: usize,
) -> Option<String> {
	let (count, size) = tally(member.iter().filter_map(|member| genre.get(*member)));

	if size == 0 || tagged == 0 {
		return None;
	}

	let mut ranked: Vec<(&str, f32, usize)> = count
		.into_iter()
		.filter(|(_, count)| *count >= MIN_TOKEN_OCCURRENCE)
		.filter_map(|(token, count)| {
			let overall = library.get(token)?;
			let share = real::of(count) / real::of(size);
			let base = real::of(*overall) / real::of(tagged);

			if share < MIN_TOKEN_SHARE {
				return None;
			}

			Some((token, share / base, count))
		})
		.collect();

	if ranked.is_empty() {
		return None;
	}

	ranked.sort_unstable_by(|a, b| b.1.total_cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(b.0)));

	Some(
		ranked
			.iter()
			.take(TOKEN_IN_NAME)
			.map(|(token, _, _)| *token)
			.collect::<Vec<_>>()
			.join(GENRE_SEPARATOR),
	)
}

fn tally<'a>(genre: impl Iterator<Item = &'a Vec<String>>) -> (HashMap<&'a str, usize>, usize) {
	let mut count: HashMap<&str, usize> = HashMap::new();
	let mut tagged = 0;

	for genre in genre {
		if genre.is_empty() {
			continue;
		}

		tagged += 1;
		for token in genre {
			*count.entry(token.as_str()).or_default() += 1;
		}
	}

	(count, tagged)
}

fn distinct(name: Vec<String>) -> Vec<String> {
	let mut seen: HashMap<String, usize> = HashMap::new();

	name.into_iter()
		.map(|name| {
			let taken = seen.entry(name.clone()).or_default();
			*taken += 1;

			if *taken == 1 {
				name
			} else {
				format!("{name} {taken}")
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn genre(token: &[&[&str]]) -> Vec<Vec<String>> {
		token
			.iter()
			.map(|token| token.iter().map(|token| (*token).to_string()).collect())
			.collect()
	}

	#[test]
	fn the_name_is_the_token_the_island_has_more_of_than_the_library() {
		let genre = genre(&[
			&["electronic", "touhou"],
			&["electronic", "touhou"],
			&["electronic"],
			&["electronic"],
			&["electronic"],
			&["electronic"],
		]);
		let (library, tagged) = tally(genre.iter());

		let name = over_represented(&[0, 1], &genre, &library, tagged);

		assert_eq!(
			name.as_deref().map(|n| n.split(GENRE_SEPARATOR).next()),
			Some(Some("touhou"))
		);
	}

	#[test]
	fn a_token_seen_once_in_an_island_does_not_name_it() {
		let genre = genre(&[&["touhou", "rare"], &["touhou"], &["pop"]]);
		let (library, tagged) = tally(genre.iter());

		let name = over_represented(&[0, 1], &genre, &library, tagged).unwrap_or_default();

		assert!(!name.contains("rare"), "{name}");
	}

	#[test]
	fn a_token_too_few_of_the_island_carry_does_not_name_it() {
		let mut token: Vec<&[&str]> = vec![&["quirk"], &["quirk"]];
		token.extend(std::iter::repeat_n::<&[&str]>(&["pop"], 58));
		let genre = genre(&token);
		let (library, tagged) = tally(genre.iter());

		let name = over_represented(&(0..60).collect::<Vec<_>>(), &genre, &library, tagged)
			.unwrap_or_default();

		assert!(!name.contains("quirk"), "{name}");
	}

	#[test]
	fn an_island_with_no_tagged_file_has_no_name() {
		let genre = genre(&[&[], &[], &["pop"]]);
		let (library, tagged) = tally(genre.iter());

		assert!(over_represented(&[0, 1], &genre, &library, tagged).is_none());
	}

	#[test]
	fn a_nameless_island_falls_back_to_its_ordinal() {
		assert_eq!(name(&[], &[vec![], vec![]]), vec!["island 0", "island 1"]);
	}

	#[test]
	fn two_islands_never_share_a_name() {
		assert_eq!(
			distinct(vec![
				"touhou".to_string(),
				"touhou".to_string(),
				"metal".to_string(),
				"touhou".to_string(),
			]),
			vec!["touhou", "touhou 2", "metal", "touhou 3"]
		);
	}

	#[test]
	fn at_most_three_tokens_make_a_name() {
		let genre = genre(&[
			&["a", "b", "c", "d", "e"],
			&["a", "b", "c", "d", "e"],
			&["z"],
		]);
		let (library, tagged) = tally(genre.iter());

		let name = over_represented(&[0, 1], &genre, &library, tagged).unwrap_or_default();

		assert_eq!(name.split(GENRE_SEPARATOR).count(), TOKEN_IN_NAME);
	}
}

use std::collections::{HashMap, HashSet};

const SEPARATOR: &str = "---";

const FAMILY_SPECIFIC: [((&str, &str), &str); 2] = [
	(("Electronic", "Hardcore"), "hardcore techno"),
	(("Rock", "Hardcore"), "hardcore punk"),
];

const FAMILY_GENRE: [(&str, &[&str]); 4] = [
	("Children's", &["children's music"]),
	("Funk / Soul", &["funk", "soul"]),
	("Folk, World, & Country", &["folk", "country"]),
	("Brass & Military", &["brass band"]),
];

const SYNONYM: [(&str, &str); 52] = [
	("drum n bass", "drum and bass"),
	("rhythm & blues", "r&b"),
	("rock & roll", "rock and roll"),
	("psy-trance", "psytrance"),
	("hi nrg", "hi-nrg"),
	("nu-disco", "nu disco"),
	("darkwave", "dark wave"),
	("minimal", "minimal techno"),
	("acid", "acid house"),
	("prog rock", "progressive rock"),
	("post rock", "post-rock"),
	("brit pop", "britpop"),
	("goth rock", "gothic rock"),
	("thrash", "thrash metal"),
	("italodance", "italo dance"),
	("bossanova", "bossa nova"),
	("fusion", "jazz fusion"),
	("rnb/swing", "new jack swing"),
	("gangsta", "gangsta rap"),
	("conscious", "conscious hip hop"),
	("jazzy hip-hop", "jazz rap"),
	("bubblegum", "bubblegum pop"),
	("bop", "bebop"),
	("post bop", "post-bop"),
	("soul-jazz", "soul jazz"),
	("space-age", "space age pop"),
	("acoustic", "acoustic rock"),
	("doo wop", "doo-wop"),
	("modern", "modern classical"),
	("contemporary", "contemporary classical"),
	("modern electric blues", "electric blues"),
	("boogie woogie", "boogie-woogie"),
	("ndw", "neue deutsche welle"),
	("power violence", "powerviolence"),
	("horror rock", "horror punk"),
	("ethereal", "ethereal wave"),
	("avantgarde", "avant-garde"),
	("afro-cuban", "afro-cuban jazz"),
	("son", "son cubano"),
	("bollywood", "filmi"),
	("beat", "beat music"),
	("crust", "crust punk"),
	("tribal", "tribal house"),
	("future jazz", "nu jazz"),
	("berlin-school", "berlin school"),
	("hip-house", "hip house"),
	("ghetto", "ghetto house"),
	("screw", "chopped and screwed"),
	("gogo", "go-go"),
	("p.funk", "p-funk"),
	("favela funk", "funk carioca"),
	("hardcore hip-hop", "hardcore hip hop"),
];

pub(super) fn genres(
	scores: &HashMap<String, f32>,
	vocabulary: &HashSet<String>,
) -> HashMap<String, f32> {
	let mut genres = HashMap::<String, f32>::new();

	for (label, &score) in scores {
		let Some((family, style)) = label.split_once(SEPARATOR) else {
			continue;
		};

		let names = family_genres(family)
			.into_iter()
			.chain([style_genre(family, style)])
			.filter(|name| vocabulary.contains(name));

		for name in names {
			genres
				.entry(name)
				.and_modify(|kept| *kept = kept.max(score))
				.or_insert(score);
		}
	}

	genres
}

fn style_genre(family: &str, style: &str) -> String {
	let name = FAMILY_SPECIFIC
		.iter()
		.find(|((f, s), _)| *f == family && *s == style)
		.map_or_else(
			|| style.trim().to_lowercase(),
			|(_, name)| (*name).to_string(),
		);

	SYNONYM
		.iter()
		.find(|(style, _)| *style == name)
		.map_or(name, |(_, genre)| (*genre).to_string())
}

fn family_genres(family: &str) -> Vec<String> {
	FAMILY_GENRE.iter().find(|(f, _)| *f == family).map_or_else(
		|| vec![family.trim().to_lowercase()],
		|(_, genres)| genres.iter().map(|genre| (*genre).to_string()).collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn vocabulary(names: &[&str]) -> HashSet<String> {
		names.iter().map(|name| (*name).to_string()).collect()
	}

	fn scores(scored: &[(&str, f32)]) -> HashMap<String, f32> {
		scored
			.iter()
			.map(|(label, score)| ((*label).to_string(), *score))
			.collect()
	}

	#[test]
	fn a_style_named_as_on_musicbrainz_maps_to_itself_lowercased() {
		let genres = genres(
			&scores(&[("Electronic---Vaporwave", 0.7)]),
			&vocabulary(&["vaporwave"]),
		);

		assert_eq!(genres.get("vaporwave"), Some(&0.7));
	}

	#[test]
	fn a_style_spelled_differently_on_musicbrainz_goes_through_its_synonym() {
		let genres = genres(
			&scores(&[("Electronic---Drum n Bass", 0.5)]),
			&vocabulary(&["drum and bass"]),
		);

		assert_eq!(genres.get("drum and bass"), Some(&0.5));
	}

	#[test]
	fn hardcore_splits_by_the_family_it_comes_from() {
		let genres = genres(
			&scores(&[("Electronic---Hardcore", 0.6), ("Rock---Hardcore", 0.3)]),
			&vocabulary(&["hardcore techno", "hardcore punk", "hardcore"]),
		);

		assert_eq!(genres.get("hardcore techno"), Some(&0.6));
		assert_eq!(genres.get("hardcore punk"), Some(&0.3));
		assert!(!genres.contains_key("hardcore"));
	}

	#[test]
	fn a_family_is_a_genre_scored_by_its_best_style() {
		let genres = genres(
			&scores(&[("Jazz---Bop", 0.4), ("Jazz---Cool Jazz", 0.6)]),
			&vocabulary(&["jazz", "bebop"]),
		);

		assert_eq!(genres.get("jazz"), Some(&0.6));
		assert_eq!(genres.get("bebop"), Some(&0.4));
	}

	#[test]
	fn a_family_spanning_several_genres_scores_each_of_them() {
		let genres = genres(
			&scores(&[("Funk / Soul---Neo Soul", 0.5)]),
			&vocabulary(&["funk", "soul", "neo soul"]),
		);

		assert_eq!(genres.get("funk"), Some(&0.5));
		assert_eq!(genres.get("soul"), Some(&0.5));
		assert_eq!(genres.get("neo soul"), Some(&0.5));
	}

	#[test]
	fn styles_landing_on_the_same_genre_keep_the_highest_score() {
		let genres = genres(
			&scores(&[
				("Hip Hop---Hardcore Hip-Hop", 0.2),
				("Rock---Hardcore Hip-Hop", 0.5),
			]),
			&vocabulary(&["hardcore hip hop"]),
		);

		assert_eq!(genres.get("hardcore hip hop"), Some(&0.5));
	}

	#[test]
	fn a_genre_musicbrainz_does_not_know_is_dropped() {
		let genres = genres(
			&scores(&[
				("Stage & Screen---Unknown Style", 0.9),
				("Not A Label", 0.9),
			]),
			&vocabulary(&["vaporwave"]),
		);

		assert_eq!(genres.len(), 0);
	}
}

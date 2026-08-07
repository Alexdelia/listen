use musicbrainz_rs::entity::alias::Alias;

pub fn other_name<'a>(alias: Option<&'a [Alias]>, title: &str) -> Option<&'a str> {
	alias
		.into_iter()
		.flatten()
		.filter(|a| a.primary == Some(true))
		.map(|a| a.name.trim())
		.find(|name| *name != title.trim())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn alias(name: &str, primary: bool) -> Alias {
		Alias {
			name: name.to_string(),
			primary: Some(primary),
			..Alias::default()
		}
	}

	#[test]
	fn the_primary_alias_that_is_not_the_title_is_the_other_name() {
		let alias = [alias("ひみつ基地", true), alias("Secret base", true)];

		assert_eq!(other_name(Some(&alias), "ひみつ基地"), Some("Secret base"));
	}

	#[test]
	fn a_non_primary_alias_is_never_the_other_name() {
		let alias = [alias("secret base", false)];

		assert_eq!(other_name(Some(&alias), "ひみつ基地"), None);
	}

	#[test]
	fn a_title_with_no_alias_stands_alone() {
		assert_eq!(other_name(None, "Secret base"), None);
	}
}

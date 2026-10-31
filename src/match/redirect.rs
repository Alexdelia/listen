use miette::{IntoDiagnostic, WrapErr};

use super::{link, verify};

const CANONICAL: &str = "<link rel=\"canonical\" href=\"";
const CANONICAL_CLOSE: char = '"';

pub(super) fn resolve(id: &str) -> miette::Result<Option<String>> {
	let url = verify::watch(id);

	let body = listen_agent::shared()
		.get(&url)
		.call()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to fetch {url}"))?
		.body_mut()
		.read_to_string()
		.into_diagnostic()
		.wrap_err_with(|| format!("failed to read {url}"))?;

	Ok(canonical(&body).filter(|replacement| replacement != id))
}

fn canonical(body: &str) -> Option<String> {
	let href = &body[body.find(CANONICAL)? + CANONICAL.len()..];

	link::video_id(&href[..href.find(CANONICAL_CLOSE)?])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn canonical_replacement() {
		let body =
			"<link rel=\"canonical\" href=\"https://music.youtube.com/watch?v=NCuUjjBt-Qs\">";

		assert_eq!(canonical(body).as_deref(), Some("NCuUjjBt-Qs"));
	}

	#[test]
	fn no_canonical() {
		assert_eq!(canonical("<title>gone</title>"), None);
	}
}

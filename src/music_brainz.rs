use const_format::concatcp;
use musicbrainz_rs::{
	MusicBrainzClient,
	api_bindium::{
		ApiClient,
		ureq::{Agent, config::Config},
	},
};

use crate::meta_brainz;

const AUTHOR: &str = author_name(env!("CARGO_PKG_AUTHORS"));

pub const CLIENT: &str = concatcp!(
	AUTHOR,
	"/",
	env!("CARGO_PKG_NAME"),
	"-",
	env!("CARGO_PKG_VERSION"),
);

pub const USER_AGENT: &str = concatcp!(CLIENT, " ( https://github.com/Alexdelia/listen )");

pub fn client() -> MusicBrainzClient {
	let agent = Agent::new_with_config(Config::builder().user_agent(USER_AGENT).build());

	MusicBrainzClient::builder()
		.api_client(
			ApiClient::builder()
				.agent(agent)
				.rate_limit(Some(meta_brainz::limiter()))
				.build(),
		)
		.build()
}

const fn author_name(authors: &str) -> &str {
	let bytes = authors.as_bytes();
	let mut end = 0;
	while end < bytes.len() && bytes[end] != b' ' {
		end += 1;
	}
	authors.split_at(end).0
}

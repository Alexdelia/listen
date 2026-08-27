use musicbrainz_rs::{MusicBrainzClient, api_bindium::ApiClient};

use crate::meta_brainz;

pub(crate) fn client() -> MusicBrainzClient {
	MusicBrainzClient::builder()
		.api_client(
			ApiClient::builder()
				.agent(listen_agent::shared().clone())
				.rate_limit(Some(meta_brainz::limiter()))
				.build(),
		)
		.build()
}

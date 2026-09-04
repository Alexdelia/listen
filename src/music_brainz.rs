use musicbrainz_rs::{MusicBrainzClient, api_bindium::ApiClient};
use ureq::{Body, RequestBuilder, http::Response, typestate::WithBody};

use crate::meta_brainz::{self, Sent};

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

pub(crate) fn post(
	url: &str,
	send: impl Fn(RequestBuilder<WithBody>) -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	meta_brainz::send(url, || send(listen_agent::status_kept().post(url)), failure)
}

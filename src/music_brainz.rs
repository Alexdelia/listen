use hmerr::ge;
use musicbrainz_rs::{MusicBrainzClient, api_bindium::ApiClient};
use ureq::{
	Body, RequestBuilder,
	http::{Response, StatusCode},
	typestate::WithBody,
};

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

pub(crate) struct Sent {
	pub status: StatusCode,
	pub body: String,
}

pub(crate) fn post(
	url: &str,
	send: impl FnOnce(RequestBuilder<WithBody>) -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	meta_brainz::block_ready();

	let mut response =
		send(listen_agent::status_kept().post(url)).map_err(|e| ge!(format!("{failure}\n{e}")))?;

	Ok(Sent {
		status: response.status(),
		body: response.body_mut().read_to_string().unwrap_or_default(),
	})
}

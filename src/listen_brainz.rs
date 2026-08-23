use hmerr::ge;
use ureq::{
	Body,
	http::{Response, StatusCode},
};

use crate::meta_brainz;

const API: &str = "https://api.listenbrainz.org/1";

pub(crate) struct Fetched {
	pub status: StatusCode,
	pub body: String,
}

pub(crate) fn get(path: &str, failure: &str) -> hmerr::Result<Fetched> {
	meta_brainz::block_ready();

	fetched(ureq::get(url(path)).call(), failure)
}

pub(crate) fn post(path: &str, body: &serde_json::Value, failure: &str) -> hmerr::Result<Fetched> {
	meta_brainz::block_ready();

	fetched(ureq::post(url(path)).send_json(body), failure)
}

fn url(path: &str) -> String {
	format!("{API}/{path}")
}

fn fetched(called: Result<Response<Body>, ureq::Error>, failure: &str) -> hmerr::Result<Fetched> {
	let mut response = called.map_err(|e| ge!(format!("{failure}\n{e}")))?;
	let status = response.status();

	Ok(Fetched {
		status,
		body: response
			.body_mut()
			.read_to_string()
			.map_err(|e| ge!(format!("{failure}\n{e}")))?,
	})
}

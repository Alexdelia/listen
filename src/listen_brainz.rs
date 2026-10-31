use miette::miette;

use crate::meta_brainz::{self, Sent};

const API: &str = "https://api.listenbrainz.org/1";

pub(crate) fn get(path: &str, failure: &str) -> miette::Result<Sent> {
	let url = url(path);

	answered(
		meta_brainz::send(
			&url,
			|| listen_agent::status_kept().get(&url).call(),
			failure,
		)?,
		failure,
	)
}

pub(crate) fn post(path: &str, body: &serde_json::Value, failure: &str) -> miette::Result<Sent> {
	let url = url(path);

	answered(
		meta_brainz::send(
			&url,
			|| listen_agent::status_kept().post(&url).send_json(body),
			failure,
		)?,
		failure,
	)
}

fn url(path: &str) -> String {
	format!("{API}/{path}")
}

fn answered(sent: Sent, failure: &str) -> miette::Result<Sent> {
	let Sent { status, body } = &sent;

	if status.is_client_error() || status.is_server_error() {
		return Err(miette!("{failure}\n{status}\n{body}"));
	}

	Ok(sent)
}

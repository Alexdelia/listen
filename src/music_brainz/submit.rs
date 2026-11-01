use miette::miette;

use crate::meta_brainz::Sent;

use super::post;

const CONTENT_TYPE: &str = "application/xml; charset=utf-8";

pub(crate) fn submit(
	bearer: &str,
	endpoint: &str,
	recording_list: &str,
	what: &str,
) -> miette::Result<()> {
	let body = format!(
		"<?xml version=\"1.0\" encoding=\"UTF-8\"?><metadata xmlns=\"http://musicbrainz.org/ns/mmd-2.0#\"><recording-list>{recording_list}</recording-list></metadata>"
	);

	let Sent {
		status,
		body: detail,
	} = post(
		&format!("{endpoint}?client={client}", client = listen_agent::CLIENT),
		|request| {
			request
				.header("content-type", CONTENT_TYPE)
				.header("authorization", format!("Bearer {bearer}"))
				.send(&body)
		},
		&format!("failed to submit {what}"),
	)?;

	if !status.is_success() {
		return Err(miette!(
			"musicbrainz refused {what} submission ({status})\n{detail}"
		));
	}

	Ok(())
}

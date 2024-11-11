use async_std::{channel::Sender, fs::remove_file};
use hmerr::ioe;

use crate::{
	channel::{Action, Status},
	entry::Entry,
};

pub async fn remove(sync: &[String], tx: Sender<Status>) {
	for entry in sync {
		let path = Entry::path_from_source(entry);

		match remove_file(&path).await {
			Ok(_) => tx.send(Status {
				action: Action::RemoveFile,
				status: Ok(()),
			}),
			Err(e) => tx.send(Status {
				action: Action::RemoveFile,
				status: Err(ioe!(path.to_string_lossy(), e).to_string()),
			}),
		}
		.await
		.expect("failed to send remove status");
	}
}

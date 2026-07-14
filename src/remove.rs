use async_std::{channel::Sender, fs::remove_file};
use hmerr::ioe;

use crate::{
	channel::{Action, Status, report},
	entry::Entry,
};

pub async fn remove(sync: &[String], tx: Sender<Status>) {
	for entry in sync {
		let path = Entry::path_from_source(entry);

		let status = remove_file(&path)
			.await
			.map_err(|e| ioe!(path.to_string_lossy(), e).to_string());

		report(
			&tx,
			Status {
				action: Action::RemoveFile,
				status,
			},
		)
		.await;
	}
}

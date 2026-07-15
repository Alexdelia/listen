use async_std::{channel::Sender, fs::remove_file};
use hmerr::ioe;

use crate::library;

use super::channel::{Action, Status, report};

pub async fn remove(sync: &[String], tx: Sender<Status>) {
	for entry in sync {
		let path = library::recording::path(entry);

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

use async_std::{channel::Sender, fs::remove_file};
use miette::{IntoDiagnostic, WrapErr};

use crate::{declaration::Source, library};

use super::channel::{Action, Status, report};

pub(super) async fn remove(sync: &[Source], tx: Sender<Status>) {
	for entry in sync {
		let path = library::recording::path(*entry);

		let status = remove_file(&path)
			.await
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot remove {}", path.display()));

		report(&tx, Action::RemoveFile, status).await;
	}
}

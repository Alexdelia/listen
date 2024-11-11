use async_std::channel::Sender;

use crate::{
	channel::{Action, Status},
	entry::Q,
	filter::SyncEntry,
};

pub async fn q(q: Q, sync: SyncEntry, tx: Sender<Status>) {
	tx.send(Status {
		action: Action::SyncPlaylist,
		status: Ok(()),
	})
	.await
	.expect("failed to send sync playlist status");
}

pub async fn playlist(playlist: String, sync: SyncEntry, tx: Sender<Status>) {
	tx.send(Status {
		action: Action::SyncPlaylist,
		status: Ok(()),
	})
	.await
	.expect("failed to send sync playlist status");
}

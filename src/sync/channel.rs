use async_std::channel::Sender;

#[derive(Debug)]
pub enum Action {
	FetchMusicBrainz,
	FetchStreaming,
	AddMetadata,

	RemoveFile,

	SyncPlaylist,

	SubmitRating(usize),
}

#[derive(Debug)]
pub struct Status {
	pub action: Action,
	pub status: Result<(), String>,
}

#[allow(
	clippy::expect_used,
	reason = "the receiver lives in progress() and is dropped only after every sender, so a send can only fail during shutdown when there is nowhere left to report to"
)]
pub async fn report(tx: &Sender<Status>, status: Status) {
	tx.send(status).await.expect("failed to send status");
}

#[derive(Debug)]
pub enum Action {
	FetchMusicBrainz,
	FetchStreaming,
	AddMetadata,

	SyncPlaylist,
}

#[derive(Debug)]
pub struct Status {
	pub action: Action,
	pub status: Result<(), String>,
}

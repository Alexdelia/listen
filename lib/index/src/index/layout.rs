pub(crate) const DIR: &str = "index";
pub(crate) const META: &str = "meta.json";
pub(crate) const RECORDING: &str = "recording.parquet";
pub(crate) const RECORDING_ARTIST: &str = "recording_artist.parquet";
pub(crate) const RECORDING_LISTENER: &str = "recording_listener.parquet";
pub(crate) const USER_LISTEN: &str = "user_listen";
pub(crate) const USER_STAT: &str = "user_stat.parquet";
pub(crate) const ARTIST_LINK: &str = "artist_link.parquet";
pub(crate) const BUCKET: u32 = 8;

pub(crate) fn shard(bucket: u32) -> String {
	format!("{bucket}.parquet")
}

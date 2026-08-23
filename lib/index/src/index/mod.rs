pub(crate) mod layout;
pub(crate) mod meta;
pub(crate) mod session;
mod state;

use std::path::{Path, PathBuf};

use listen_cache as cache;

use layout::{
	ARTIST_LINK, DIR, RECORDING, RECORDING_ARTIST, RECORDING_LISTENER, USER_LISTEN, USER_STAT,
};

pub(crate) use state::{built, predates_listener, predates_stat, scanned};

pub use meta::{Gap, Meta};

pub(crate) fn dir() -> hmerr::Result<PathBuf> {
	Ok(cache::root()?.join(DIR))
}

pub struct Index {
	pub db: duckdb::Connection,
	pub meta: Meta,
}

pub(crate) fn open(dir: &Path) -> hmerr::Result<Index> {
	let meta = meta::read(dir)?;
	let db = session::of(dir)?;

	db.execute_batch(&format!(
		r"
create view recording as select * from read_parquet('{dir}/{RECORDING}');
create view recording_artist as select * from read_parquet('{dir}/{RECORDING_ARTIST}');
create view recording_listener as select * from read_parquet('{dir}/{RECORDING_LISTENER}');
create view user_listen as select * from read_parquet('{dir}/{USER_LISTEN}/*.parquet');
create view user_stat as select * from read_parquet('{dir}/{USER_STAT}');
create view artist_link as select * from read_parquet('{dir}/{ARTIST_LINK}');
",
		dir = dir.display()
	))?;

	Ok(Index { db, meta })
}

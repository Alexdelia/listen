pub(super) fn declare(db: &duckdb::Connection, allow: bool) -> hmerr::Result<()> {
	db.execute_batch(if allow { NOTHING_KNOWN } else { KNOWN_ARTIST })?;

	Ok(())
}

const NOTHING_KNOWN: &str = "create or replace temp table known_artist (artist_mbid uuid);";

const KNOWN_ARTIST: &str = r"
create or replace temp table known_artist as
with seed_artist as (
	select distinct ra.artist_mbid
	from declared d
	join recording r on r.mbid = d.mbid::uuid
	join recording_artist ra using (recording_id)
)
select artist_mbid from seed_artist
union
select al.related_mbid
from artist_link al
semi join seed_artist s on s.artist_mbid = al.artist_mbid;
";

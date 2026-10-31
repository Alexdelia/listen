use std::collections::HashMap;

use miette::IntoDiagnostic;

use crate::declaration::Source;

use super::collect::Pick;

pub(in crate::recommend) fn attach(
	db: &duckdb::Connection,
	pick: &mut [Pick],
) -> miette::Result<()> {
	let mut artist = of(db, pick.iter().map(|pick| pick.recommendation.mbid))?;

	for pick in pick {
		pick.artist = artist.remove(&pick.recommendation.mbid).unwrap_or_default();
	}

	Ok(())
}

fn of(
	db: &duckdb::Connection,
	recording: impl Iterator<Item = Source>,
) -> miette::Result<HashMap<Source, Vec<Source>>> {
	db.execute_batch("create or replace temp table trace_pick (mbid varchar);")
		.into_diagnostic()?;
	{
		let mut appender = db.appender("trace_pick").into_diagnostic()?;
		for mbid in recording {
			appender
				.append_row(duckdb::params![mbid.to_string()])
				.into_diagnostic()?;
		}
		appender.flush().into_diagnostic()?;
	}

	let mut statement = db
		.prepare(
			r"
select p.mbid, ra.artist_mbid::varchar
from trace_pick p
join recording r on r.mbid = p.mbid::uuid
join recording_artist ra using (recording_id)
order by 1, 2
",
		)
		.into_diagnostic()?;
	let mut row = statement.query([]).into_diagnostic()?;
	let mut artist: HashMap<Source, Vec<Source>> = HashMap::new();

	while let Some(row) = row.next().into_diagnostic()? {
		let recording: String = row.get(0).into_diagnostic()?;
		let credited: String = row.get(1).into_diagnostic()?;

		let (Ok(recording), Ok(credited)) = (recording.parse(), credited.parse()) else {
			continue;
		};

		artist.entry(recording).or_default().push(credited);
	}

	Ok(artist)
}

#[cfg(test)]
mod tests {
	use super::*;

	const RECORDING: &str = "00000000-0000-0000-0000-000000000001";
	const ARTIST: &str = "00000000-0000-0000-0000-0000000000a1";
	const FEATURED: &str = "00000000-0000-0000-0000-0000000000a2";
	const UNINDEXED: &str = "00000000-0000-0000-0000-000000000009";

	fn db() -> duckdb::Connection {
		let db = duckdb::Connection::open_in_memory().unwrap();
		db.execute_batch(&format!(
			r"
create table recording (recording_id uinteger, mbid uuid);
create table recording_artist (recording_id uinteger, artist_mbid uuid);
insert into recording values (0, '{RECORDING}');
insert into recording_artist values (0, '{ARTIST}'), (0, '{FEATURED}');
"
		))
		.unwrap();
		db
	}

	fn mbid(text: &str) -> Source {
		text.parse().unwrap()
	}

	#[test]
	fn a_recording_carries_every_artist_credited_on_it() {
		let artist = of(&db(), [mbid(RECORDING)].into_iter()).unwrap();

		assert_eq!(
			artist.get(&mbid(RECORDING)),
			Some(&vec![mbid(ARTIST), mbid(FEATURED)])
		);
	}

	#[test]
	fn a_recording_absent_from_the_index_has_no_artist() {
		let artist = of(&db(), [mbid(UNINDEXED)].into_iter()).unwrap();

		assert!(artist.is_empty());
	}
}

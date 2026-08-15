use std::{collections::HashMap, path::Path};

use crate::declaration::{Q, Source, parse, value};

use super::{attraction, index::Index, real};

pub(super) struct Listener {
	pub user: u32,
	pub weight: f32,
}

pub(super) struct Seed {
	pub mbid: Source,
	pub q: Q,
	pub listener: Vec<Listener>,
}

impl Seed {
	pub(super) fn weight(&self) -> f32 {
		value::weight(self.q)
	}
}

pub(super) struct Library {
	pub seed: Vec<Seed>,
	pub user: Vec<i64>,
	pub declared: Vec<Source>,
}

impl Library {
	pub(super) fn unsupported(&self) -> usize {
		self.declared.len() - self.seed.len()
	}

	pub(super) fn q(&self) -> f32 {
		if self.seed.is_empty() {
			return 0.0;
		}

		self.seed.iter().map(|seed| f32::from(seed.q)).sum::<f32>() / real::of(self.seed.len())
	}
}

pub(super) fn load(path: &Path, index: &Index) -> hmerr::Result<Library> {
	let entry = parse::parse(path)?;
	declare(index, &entry)?;

	let mut listen: HashMap<Source, Vec<Listener>> = HashMap::new();
	let mut dense: HashMap<i64, u32> = HashMap::new();
	let mut user = Vec::new();

	let mut statement = index.db.prepare(&format!(
		r"
select
	r.mbid::varchar,
	ul.user_id::bigint,
	{weight}(ul.plays, s.center, s.low, s.high)::float
from user_listen ul
join user_stat s using (user_id)
join recording r using (recording_id)
semi join declared d on d.mbid::uuid = r.mbid
",
		weight = attraction::WEIGHT
	))?;
	let mut row = statement.query([])?;

	while let Some(row) = row.next()? {
		let mbid: String = row.get(0)?;
		let listener: i64 = row.get(1)?;
		let weight: f32 = row.get(2)?;

		let Ok(mbid) = mbid.parse() else {
			continue;
		};

		let dense = *dense.entry(listener).or_insert_with(|| {
			user.push(listener);
			u32::try_from(user.len() - 1).unwrap_or(u32::MAX)
		});

		listen.entry(mbid).or_default().push(Listener {
			user: dense,
			weight,
		});
	}

	let seed = entry
		.iter()
		.filter_map(|entry| {
			let mut listen = listen.remove(&entry.s)?;
			listen.sort_unstable_by_key(|listener| listener.user);

			Some(Seed {
				mbid: entry.s,
				q: entry.q,
				listener: listen,
			})
		})
		.collect();

	Ok(Library {
		seed,
		user,
		declared: entry.iter().map(|entry| entry.s).collect(),
	})
}

fn declare(index: &Index, entry: &[crate::declaration::Entry]) -> hmerr::Result<()> {
	index
		.db
		.execute_batch("create or replace temp table declared (mbid varchar, q utinyint);")?;

	let mut appender = index.db.appender("declared")?;
	for entry in entry {
		appender.append_row(duckdb::params![entry.s.to_string(), entry.q])?;
	}
	appender.flush()?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::{super::index::Meta, *};

	const DECLARED: &str = "00000000-0000-0000-0000-000000000001";
	const CENTER_PLAY: f32 = 10.0;
	const HIGH_PLAY: f32 = 100.0;

	fn index(listen: &[(u32, u32)]) -> Index {
		let db = duckdb::Connection::open_in_memory().unwrap();
		attraction::declare(&db).unwrap();

		db.execute_batch(&format!(
			r"
create table recording (recording_id uinteger, mbid uuid, global_plays uinteger);
create table user_listen (user_id uinteger, recording_id uinteger, plays usmallint);
create table user_stat (user_id uinteger, center float, low float, high float, recording uinteger);
insert into recording values (0, '{DECLARED}', 1000);
insert into user_stat select range, {center}, 0, {high}, 100 from range({member});
{listen}
",
			center = CENTER_PLAY.ln(),
			high = HIGH_PLAY.ln(),
			member = listen.len(),
			listen = if listen.is_empty() {
				String::new()
			} else {
				format!(
					"insert into user_listen values {row};",
					row = listen
						.iter()
						.map(|(user, plays)| format!("({user}, 0, {plays})"))
						.collect::<Vec<_>>()
						.join(",")
				)
			},
		))
		.unwrap();

		Index {
			db,
			meta: Meta {
				built: String::new(),
				dump: String::new(),
				own: None,
				user: 0,
				recording: 1,
				user_listen: 0,
			},
		}
	}

	fn declaration(name: &str) -> std::path::PathBuf {
		let path = std::env::temp_dir().join(format!("declarative_listen_seed_{name}.ron"));
		fs::write(&path, format!("[(s: \"{DECLARED}\", q: 4, playlist: [])]")).unwrap();

		path
	}

	#[test]
	fn every_listener_of_a_declared_recording_carries_its_own_attraction() {
		let path = declaration("weight");
		let library = load(&path, &index(&[(0, 100), (1, 1)])).unwrap();
		let seed = library.seed.first().unwrap();

		assert_eq!(seed.listener.len(), 2);
		assert!(seed.listener[0].weight > 0.0);
		assert!(seed.listener[1].weight < 0.0);
		let _ = fs::remove_file(&path);
	}

	#[test]
	fn a_declared_recording_nobody_listened_to_is_unsupported() {
		let path = declaration("unsupported");
		let library = load(&path, &index(&[])).unwrap();

		assert!(library.seed.is_empty());
		assert_eq!(library.unsupported(), 1);
		let _ = fs::remove_file(&path);
	}
}

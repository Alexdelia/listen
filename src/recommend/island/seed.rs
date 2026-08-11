use std::{collections::HashMap, path::Path};

use crate::declaration::{Q, Source, parse, value};

use super::index::Index;

const DELIBERATE_PLAY: i32 = 3;

pub(super) struct Seed {
	pub mbid: Source,
	pub q: Q,
	pub listener: Vec<u32>,
	pub deliberate: Vec<u32>,
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
}

pub(super) fn load(path: &Path, index: &Index) -> hmerr::Result<Library> {
	let entry = parse::parse(path)?;
	declare(index, &entry)?;

	let mut listen: HashMap<Source, Vec<(u32, i32)>> = HashMap::new();
	let mut dense: HashMap<i64, u32> = HashMap::new();
	let mut user = Vec::new();

	let mut statement = index.db.prepare(
		r"
select r.mbid::varchar, ul.user_id::bigint, ul.plays::integer
from user_listen ul
join recording r using (recording_id)
semi join declared d on d.mbid::uuid = r.mbid
",
	)?;
	let mut row = statement.query([])?;

	while let Some(row) = row.next()? {
		let mbid: String = row.get(0)?;
		let listener: i64 = row.get(1)?;
		let plays: i32 = row.get(2)?;

		let Ok(mbid) = mbid.parse() else {
			continue;
		};

		let dense = *dense.entry(listener).or_insert_with(|| {
			user.push(listener);
			u32::try_from(user.len() - 1).unwrap_or(u32::MAX)
		});

		listen.entry(mbid).or_default().push((dense, plays));
	}

	let seed = entry
		.iter()
		.filter_map(|entry| {
			let mut listen = listen.remove(&entry.s)?;
			listen.sort_unstable();

			Some(Seed {
				mbid: entry.s,
				q: entry.q,
				deliberate: listen
					.iter()
					.filter(|(_, plays)| *plays >= DELIBERATE_PLAY)
					.map(|(user, _)| *user)
					.collect(),
				listener: listen.into_iter().map(|(user, _)| user).collect(),
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

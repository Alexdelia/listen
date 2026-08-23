mod artist;
mod library;
mod pool;
mod recording;
mod recording_listener;
mod scan;
mod seed;
mod stage;
mod user_listen;
mod user_stat;
mod work;

use std::path::Path;

use ansi::abbrev::{B, D, F, G};
use chrono::Utc;

use listen_cache as cache;

use crate::Seed;

use super::{dump::Listen, index, listener, parallel, progress};

use scan::Scan;

pub(super) fn run(dir: &Path, dump: &Listen, declared: &[Seed]) -> hmerr::Result<()> {
	let listener = pool::Listener {
		named: named()?,
		known: index::meta::own(dir),
	};
	let work = work::open(dir, &dump.name)?;

	announce(declared.len());

	let scan = Scan::of(&work, &dump.dir)?;

	seed::declare(&scan, declared)?;
	let library = library::of(&scan)?;

	let (recording, pool) = parallel::both(
		|| recording::of(&scan, &work, &library),
		|| pool::of(&scan, &library, declared.len(), listener),
	)?;

	work::exclude(&work, pool.own)?;

	let ((), (), row) = parallel::all(
		|| artist::of(&scan, &work, &recording),
		|| user_stat::of(&scan, &work, &library, &pool),
		|| user_listen::of(&scan, &work, &library, &pool, &recording),
	)?;

	recording_listener::of(&scan, &work)?;

	let meta = index::Meta {
		built: Utc::now().date_naive().to_string(),
		dump: dump.name.clone(),
		own: Some(pool.own),
		reached: None,
		gap: Vec::new(),
		absorbed: 0,
		user: scan.count(&pool.path)?,
		recording: scan.count(&recording)?,
		user_listen: row,
	};

	drop(scan);
	work::publish(&work, dir, &meta)?;
	work::release(&work);

	progress::say(format!("{G}index built{D}"));

	Ok(())
}

fn named() -> hmerr::Result<Option<u32>> {
	let Some(username) = cache::username::read()? else {
		return Ok(None);
	};

	listener::of(&username)
}

fn announce(declared: usize) {
	progress::say(format!(
		"\n{F}building index from {B}{declared}{D}{F} declared recording, \
		may be long, once per dump{D}\n"
	));
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::{
		super::index::layout::{RECORDING, RECORDING_ARTIST, USER_LISTEN, USER_STAT},
		*,
	};

	const DECLARED: usize = 10;
	const SHARED: usize = 6;
	const OWN: u32 = 1;
	const POOL_USER: u32 = 5;
	const OTHER_RECORDING: usize = 25;

	fn mbid(recording: usize) -> String {
		format!("00000000-0000-0000-0000-{recording:012x}")
	}

	fn artist(recording: usize) -> String {
		format!("11111111-0000-0000-0000-{recording:012x}")
	}

	fn declaration() -> Vec<Seed> {
		(0..DECLARED)
			.map(|recording| seed(recording, recording % 5))
			.collect()
	}

	fn seed(recording: usize, q: usize) -> Seed {
		Seed {
			mbid: mbid(recording).parse().unwrap(),
			q: u8::try_from(q).unwrap(),
		}
	}

	fn listen(user: u32, recording: usize, plays: usize) -> String {
		(0..plays)
			.map(|_| {
				format!(
					"({user}, '{mbid}', ['{artist}'])",
					mbid = mbid(recording),
					artist = artist(recording)
				)
			})
			.collect::<Vec<_>>()
			.join(",")
	}

	fn dump(dir: &Path) -> Listen {
		let into = dir.join("listen");
		let _ = fs::create_dir_all(&into);

		let mut row = Vec::new();
		for recording in 0..DECLARED {
			row.push(listen(OWN, recording, 5));
		}
		for recording in DECLARED..DECLARED + OTHER_RECORDING {
			row.push(listen(OWN, recording, 3));
		}
		for user in 0..POOL_USER {
			let user = OWN + 1 + user;
			for recording in 0..SHARED {
				row.push(listen(user, recording, 4));
			}
			for recording in DECLARED..DECLARED + OTHER_RECORDING {
				row.push(listen(user, recording, 3));
			}
		}

		let db = duckdb::Connection::open_in_memory().unwrap();
		db.execute_batch(&format!(
			"copy (select * from (values {row}) as t(user_id, recording_mbid, artist_credit_mbids)) \
			to '{into}/0.parquet' (format parquet);",
			row = row.join(","),
			into = into.display()
		))
		.unwrap();

		Listen {
			dir: into,
			name: "20260712-000004".to_string(),
		}
	}

	fn count(dir: &Path, of: &str) -> i64 {
		let db = duckdb::Connection::open_in_memory().unwrap();

		db.query_row(
			&format!(
				"select count(*)::bigint from read_parquet('{of}')",
				of = dir.join(of).display()
			),
			[],
			|row| row.get(0),
		)
		.unwrap()
	}

	fn built(name: &str) -> (PathBuf, index::Meta) {
		let dir = crate::scratch::of("build", name);
		let index = dir.join("index");
		let _ = fs::create_dir_all(&index);

		run(&index, &dump(&dir), &declaration()).unwrap();
		let meta: index::Meta =
			serde_json::from_str(&fs::read_to_string(index.join(index::layout::META)).unwrap())
				.unwrap();

		(dir, meta)
	}

	#[test]
	fn a_dump_becomes_an_index_that_holds_every_recording_anyone_played() {
		let (dir, meta) = built("whole");
		let index = dir.join("index");

		assert_eq!(
			count(&index, RECORDING),
			i64::try_from(DECLARED + OTHER_RECORDING).unwrap_or_default()
		);
		assert!(count(&index, RECORDING_ARTIST) > 0);
		assert_eq!(meta.own, Some(OWN));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn the_pool_holds_every_listener_but_the_one_whose_library_is_declared() {
		let (dir, meta) = built("pool");
		let index = dir.join("index");

		assert_eq!(meta.user, u64::from(POOL_USER));
		assert_eq!(count(&index, USER_STAT), i64::from(POOL_USER));
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn own_listens_never_reach_the_index() {
		let (dir, _) = built("own");
		let db = duckdb::Connection::open_in_memory().unwrap();

		let own: i64 = db
			.query_row(
				&format!(
					"select count(*)::bigint from read_parquet('{listen}/*.parquet') where user_id = {OWN}",
					listen = dir.join("index").join(USER_LISTEN).display()
				),
				[],
				|row| row.get(0),
			)
			.unwrap();

		assert_eq!(own, 0);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_build_that_dies_partway_leaves_the_index_it_would_have_replaced() {
		let (dir, meta) = built("survive");
		let index = dir.join("index");
		let listen = count(&index, &format!("{USER_LISTEN}/*.parquet"));

		let torn = dir.join("torn");
		let _ = fs::create_dir_all(&torn);
		let _ = fs::write(torn.join("0.parquet"), b"not a parquet footer");
		let dead = Listen {
			dir: torn,
			name: "20260809-000003".to_string(),
		};

		assert!(run(&index, &dead, &declaration()).is_err());

		assert_eq!(count(&index, &format!("{USER_LISTEN}/*.parquet")), listen);
		assert_eq!(count(&index, USER_STAT), i64::from(POOL_USER));
		assert!(count(&index, RECORDING) > 0);
		let held: index::Meta =
			serde_json::from_str(&fs::read_to_string(index.join(index::layout::META)).unwrap())
				.unwrap();
		assert_eq!(held.dump, meta.dump);
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_second_build_of_the_same_dump_under_another_declaration_holds_the_same_listens() {
		let (dir, meta) = built("stable");
		let index = dir.join("index");
		let listen = count(&index, &format!("{USER_LISTEN}/*.parquet"));

		run(&index, &dump(&dir), &[seed(0, 4)]).unwrap();

		assert_eq!(count(&index, &format!("{USER_LISTEN}/*.parquet")), listen);
		assert_eq!(
			count(&index, RECORDING),
			i64::try_from(DECLARED + OTHER_RECORDING).unwrap_or_default()
		);
		assert_eq!(meta.own, Some(OWN));
		let _ = fs::remove_dir_all(&dir);
	}
}

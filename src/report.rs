use ansi::abbrev::{B, CYA, D, G, M, R};

use crate::filter::{GroupedEntry, SyncEntry};

pub fn report(sync: &GroupedEntry<SyncEntry>) -> bool {
	let mut ret = false;

	ret |= single_report("file", &sync.fs);

	let mut q = sync.q.iter().collect::<Vec<_>>();
	if !q.is_empty() {
		section("q");
		q.sort_by_key(|(q, _)| *q);
		for (q, update) in q {
			ret |= single_report(&q.to_string(), update);
		}
	}

	let mut playlist = sync.playlist.iter().collect::<Vec<_>>();
	if !playlist.is_empty() {
		section("playlist");
		playlist.sort_by_key(|(playlist, _)| *playlist);
		for (playlist, update) in playlist {
			ret |= single_report(playlist, update);
		}
	}

	ret
}

fn section(title: &str) {
	println!("\n# {B}{M}{title}{D}");
}

const LEN_REPORT_LIMIT: usize = 4;

fn single_report(title: &str, update: &SyncEntry) -> bool {
	if update.add.is_empty() && update.remove.is_empty() {
		return false;
	}

	println!("{B}{CYA}{title}{D}:");

	if update.add.len() > LEN_REPORT_LIMIT {
		println!("{B}{G}+ {len} entry{D}", len = update.add.len());
	} else {
		for source in &update.add {
			println!("{B}{G}+ {source}{D}");
		}
	}
	if update.remove.len() > LEN_REPORT_LIMIT {
		println!("{B}{R}- {len} entry{D}", len = update.remove.len());
	} else {
		for source in &update.remove {
			println!("{B}{R}- {source}{D}");
		}
	}

	!update.remove.is_empty()
}

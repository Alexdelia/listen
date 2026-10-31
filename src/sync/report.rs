use console::{Style, style};

use crate::format;

use super::filter::{GroupedEntry, SyncEntry};

pub(super) fn report(sync: &GroupedEntry<SyncEntry>) -> bool {
	let mut ret = false;

	ret |= single_report("file", Style::new().cyan(), &sync.fs);

	let mut q = sync.q.iter().collect::<Vec<_>>();
	if q.iter().any(|(_, update)| update.changed()) {
		section("q");
		q.sort_by_key(|(q, _)| *q);
		for (q, update) in q {
			ret |= single_report(&q.to_string(), format::q_style(*q), update);
		}
	}

	let mut playlist = sync.playlist.iter().collect::<Vec<_>>();
	if playlist.iter().any(|(_, update)| update.changed()) {
		section("playlist");
		playlist.sort_by_key(|(playlist, _)| *playlist);
		for (playlist, update) in playlist {
			ret |= single_report(playlist, Style::new().cyan(), update);
		}
	}

	ret
}

fn section(title: &str) {
	println!("\n# {}", style(&title).bold().magenta());
}

const LEN_REPORT_LIMIT: usize = 4;

fn single_report(title: &str, color: Style, update: &SyncEntry) -> bool {
	if !update.changed() {
		return false;
	}

	println!("{}:", color.bold().apply_to(title));

	if update.add.len() > LEN_REPORT_LIMIT {
		println!(
			"{}",
			style(format!("+ {len} entry", len = update.add.len()))
				.bold()
				.green()
		);
	} else {
		for source in &update.add {
			println!("{}", style(format!("+ {source}")).bold().green());
		}
	}
	if update.remove.len() > LEN_REPORT_LIMIT {
		println!(
			"{}",
			style(format!("- {len} entry", len = update.remove.len()))
				.bold()
				.red()
		);
	} else {
		for source in &update.remove {
			println!("{}", style(format!("- {source}")).bold().red());
		}
	}

	!update.remove.is_empty()
}

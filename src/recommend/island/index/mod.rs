mod absorb;
mod board;
mod build;
mod dump;
mod keep;
mod open;
mod parallel;
mod partial;
mod progress;
mod query;
mod recording_listener;
mod shard;
mod user_stat;
mod work;

use std::path::Path;

use ansi::abbrev::{B, D, F};

use dump::Listen;

pub(super) use open::{Index, Meta};

pub(super) fn ready() -> bool {
	open::dir().is_ok_and(|dir| open::built(&dir))
}

pub(super) fn ensure(declaration: &Path) -> hmerr::Result<Index> {
	let dir = open::dir()?;

	if let Some(listen) = to_build_from(&dir)? {
		rebuilt(&dir, &listen, declaration)?;
	} else {
		user_stat::derive(&dir)?;

		if let Some(listen) = repaired(&dir)? {
			rebuilt(&dir, &listen, declaration)?;
		}
	}

	absorbed(&dir)?;
	recording_listener::derive(&dir)?;

	dump::artist_link(&dir.join(open::ARTIST_LINK))?;

	open::open(&dir)
}

fn to_build_from(dir: &Path) -> hmerr::Result<Option<Listen>> {
	if !open::scanned(dir) {
		return dump::listen().map(Some);
	}

	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	let built = open::meta(dir)?.dump;

	let Some(reason) = worth_rebuilding(dir, &listen, &built) else {
		return Ok(None);
	};

	Ok(asked(&listen, &reason)?.then_some(listen))
}

fn worth_rebuilding(dir: &Path, listen: &Listen, built: &str) -> Option<String> {
	if newer(listen, built) {
		return Some(format!("newer than the index built from {B}{built}{D}{F}"));
	}

	missing(dir).map(|part| format!("the index it was built for predates its {part}"))
}

fn missing(dir: &Path) -> Option<&'static str> {
	if open::predates_stat(dir) {
		return Some("listener stat");
	}

	if open::predates_listener(dir) {
		return Some("listener count");
	}

	None
}

fn newer(listen: &Listen, built: &str) -> bool {
	dump::reach(&listen.name)
		.ok()
		.zip(dump::reach(built).ok())
		.is_none_or(|(unpacked, built)| unpacked > built)
}

fn asked(listen: &Listen, reason: &str) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}listen dump {B}{name}{D}{F} unpacked, {reason}, \
		rebuilding replaces the index and may be long{D}",
		name = listen.name
	));

	progress::ask("rebuild index", true)
}

fn rebuilt(dir: &Path, listen: &Listen, declaration: &Path) -> hmerr::Result<()> {
	build::run(dir, listen, declaration)?;

	dump::discard(listen)
}

fn repaired(dir: &Path) -> hmerr::Result<Option<Listen>> {
	let meta = open::meta(dir)?;

	if meta.gap.is_empty() {
		return Ok(None);
	}

	let Some(dump) = listed(dump::repairable(&meta.dump)).flatten() else {
		return Ok(None);
	};

	dump::repair(&dump)
}

fn absorbed(dir: &Path) -> hmerr::Result<()> {
	let meta = open::meta(dir)?;

	let Some(pending) = listed(dump::pending(meta.covered())) else {
		return Ok(());
	};

	if pending.is_empty() {
		return Ok(());
	}

	absorb::run(dir, &meta, &pending)
}

fn listed<T>(published: hmerr::Result<T>) -> Option<T> {
	match published {
		Ok(published) => Some(published),
		Err(e) => {
			progress::say(format!(
				"{F}cannot read what is published, keeping the index as it stands{D}\n{e}"
			));

			None
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use super::*;

	const BUILT: &str = "2026-07-12 00:00:04.001868+00:00";

	fn listen(name: &str) -> Listen {
		Listen {
			dir: PathBuf::new(),
			name: name.to_string(),
		}
	}

	#[test]
	fn the_dump_an_index_was_built_from_is_never_one_to_rebuild_from() {
		assert!(!newer(&listen(BUILT), BUILT));
	}

	#[test]
	fn a_dump_reaching_past_the_index_baseline_is_one_to_rebuild_from() {
		assert!(newer(&listen("2026-08-16 00:00:03.000000+00:00"), BUILT));
	}

	#[test]
	fn a_dump_older_than_the_index_baseline_is_left_where_it_lies() {
		assert!(!newer(&listen("2026-06-01 00:00:03.000000+00:00"), BUILT));
	}

	#[test]
	fn a_dump_whose_name_says_nothing_is_still_offered() {
		assert!(newer(&listen("listen"), BUILT));
	}

	fn whole(name: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!("declarative_listen_ensure_{name}"));
		let _ = fs::remove_dir_all(&dir);
		let into = dir.join(open::USER_LISTEN);
		let _ = fs::create_dir_all(&into);

		for part in [
			open::RECORDING,
			open::RECORDING_ARTIST,
			open::RECORDING_LISTENER,
			open::USER_STAT,
			open::META,
		] {
			let _ = fs::write(dir.join(part), b"built");
		}
		for bucket in 0..open::BUCKET {
			let _ = fs::write(into.join(open::shard(bucket)), b"built");
		}

		dir
	}

	#[test]
	fn an_index_holding_every_part_is_not_rebuilt_from_the_dump_it_was_built_from() {
		let dir = whole("whole");

		assert!(worth_rebuilding(&dir, &listen(BUILT), BUILT).is_none());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_predating_the_listener_count_is_offered_the_dump_it_was_built_from() {
		let dir = whole("uncounted");
		let _ = fs::remove_file(dir.join(open::RECORDING_LISTENER));

		let reason = worth_rebuilding(&dir, &listen(BUILT), BUILT).unwrap_or_default();

		assert!(reason.contains("listener count"), "{reason}");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_predating_the_listener_stat_is_offered_the_dump_it_was_built_from() {
		let dir = whole("unstated");
		let _ = fs::remove_file(dir.join(open::USER_STAT));

		let reason = worth_rebuilding(&dir, &listen(BUILT), BUILT).unwrap_or_default();

		assert!(reason.contains("listener stat"), "{reason}");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_newer_dump_is_offered_to_an_index_holding_every_part() {
		let dir = whole("newer");

		let reason = worth_rebuilding(&dir, &listen("2026-08-16 00:00:03.000000+00:00"), BUILT)
			.unwrap_or_default();

		assert!(reason.contains(BUILT), "{reason}");
		let _ = fs::remove_dir_all(&dir);
	}
}

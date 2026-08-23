mod absorb;
mod board;
mod build;
pub mod decide;
mod dump;
mod index;
mod keep;
mod listener;
pub mod own;
mod parallel;
mod part;
mod partial;
mod play;
mod progress;
mod query;
mod recording_listener;
#[cfg(test)]
mod scratch;
mod shard;
pub mod user_stat;
mod work;

use std::path::Path;

use uuid::Uuid;

use ansi::abbrev::{B, D, F};

use dump::Listen;

pub use decide::Decide;
pub use index::{Gap, Index, Meta};

pub struct Seed {
	pub mbid: Uuid,
	pub q: u8,
}

#[must_use]
pub fn ready() -> bool {
	index::dir().is_ok_and(|dir| index::built(&dir))
}

pub fn ensure(declared: &[Seed], decide: &dyn Decide) -> hmerr::Result<Index> {
	let dir = index::dir()?;

	if let Some(listen) = to_build_from(&dir, decide)? {
		rebuilt(&dir, &listen, declared)?;
	} else {
		user_stat::derive(&dir)?;

		if let Some(listen) = repaired(&dir, decide)? {
			rebuilt(&dir, &listen, declared)?;
		}
	}

	absorbed(&dir, decide)?;
	recording_listener::derive(&dir)?;

	dump::artist_link(&dir.join(index::layout::ARTIST_LINK), decide)?;

	index::open(&dir)
}

fn to_build_from(dir: &Path, decide: &dyn Decide) -> hmerr::Result<Option<Listen>> {
	if !index::scanned(dir) {
		return dump::listen(decide).map(Some);
	}

	let Some(listen) = dump::unpacked()? else {
		return Ok(None);
	};

	let built = index::meta::read(dir)?.dump;

	let Some(reason) = worth_rebuilding(dir, &listen, &built) else {
		return Ok(None);
	};

	Ok(asked(&listen, &reason, decide)?.then_some(listen))
}

fn worth_rebuilding(dir: &Path, listen: &Listen, built: &str) -> Option<String> {
	if newer(listen, built) {
		return Some(format!("newer than the index built from {B}{built}{D}{F}"));
	}

	index::predates_stat(dir)
		.then(|| "the index it was built for predates its listener stat".to_owned())
}

fn newer(listen: &Listen, built: &str) -> bool {
	dump::reach(&listen.name)
		.ok()
		.zip(dump::reach(built).ok())
		.is_none_or(|(unpacked, built)| unpacked > built)
}

fn asked(listen: &Listen, reason: &str, decide: &dyn Decide) -> hmerr::Result<bool> {
	progress::say(format!(
		"\n{F}listen dump {B}{name}{D}{F} unpacked, {reason}, \
		rebuilding replaces the index, resets what it absorbed and may be long{D}",
		name = listen.name
	));

	progress::confirm(decide, "rebuild index", true)
}

fn rebuilt(dir: &Path, listen: &Listen, declared: &[Seed]) -> hmerr::Result<()> {
	build::run(dir, listen, declared)?;

	dump::discard(listen)
}

fn repaired(dir: &Path, decide: &dyn Decide) -> hmerr::Result<Option<Listen>> {
	let meta = index::meta::read(dir)?;

	if meta.gap.is_empty() {
		return Ok(None);
	}

	let Some(dump) = listed(dump::repairable(&meta.dump)).flatten() else {
		return Ok(None);
	};

	dump::repair(&dump, decide)
}

fn absorbed(dir: &Path, decide: &dyn Decide) -> hmerr::Result<()> {
	let meta = index::meta::read(dir)?;

	let Some(pending) = listed(dump::pending(meta.covered())) else {
		return Ok(());
	};

	if pending.is_empty() {
		return Ok(());
	}

	absorb::run(dir, &meta, &pending, decide)
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
		let dir = crate::scratch::of("ensure", name);
		let into = dir.join(index::layout::USER_LISTEN);
		let _ = fs::create_dir_all(&into);

		for part in [
			index::layout::RECORDING,
			index::layout::RECORDING_ARTIST,
			index::layout::RECORDING_LISTENER,
			index::layout::USER_STAT,
			index::layout::META,
		] {
			let _ = fs::write(dir.join(part), b"built");
		}
		for bucket in 0..index::layout::BUCKET {
			let _ = fs::write(into.join(index::layout::shard(bucket)), b"built");
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
	fn an_index_predating_the_listener_count_counts_it_where_it_stands_rather_than_rebuild() {
		let dir = whole("uncounted");
		let _ = fs::remove_file(dir.join(index::layout::RECORDING_LISTENER));

		assert!(worth_rebuilding(&dir, &listen(BUILT), BUILT).is_none());
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn an_index_predating_the_listener_stat_is_offered_the_dump_it_was_built_from() {
		let dir = whole("unstated");
		let _ = fs::remove_file(dir.join(index::layout::USER_STAT));

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

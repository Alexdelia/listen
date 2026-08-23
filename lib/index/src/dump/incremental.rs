use std::{
	fs,
	path::{Path, PathBuf},
};

use ansi::abbrev::{B, D, R};
use hmerr::{GenericError, ge, ioe};
use indicatif::ProgressBar;

use super::{super::keep, listen, rsync, space, stamp};

const MODULE: &str = "listenbrainz/incremental";
const SUFFIX: &str = "-incremental";
const ARCHIVE: &str = "listenbrainz-spark-dump-";
const EXT: &str = ".tar";
const UNPACKED: &str = "incremental";
const START: &str = "START_TIMESTAMP";
const END: &str = "END_TIMESTAMP";
const SEQUENCE: &str = "SCHEMA_SEQUENCE";
const SEQUENCE_UNDERSTOOD: &str = "1";

pub(crate) struct Pending {
	pub name: String,
	pub archive: String,
	pub size: u64,
	pub reach: u64,
}

pub(crate) struct Incremental {
	pub dir: PathBuf,
	pub name: String,
	pub start: String,
	pub end: String,
}

pub(super) fn pending(covered: &str) -> hmerr::Result<Vec<Pending>> {
	let reached = stamp::reach(covered)?;
	let url = format!("{host}/{MODULE}/", host = rsync::HOST);

	let mut found: Vec<Pending> = rsync::beneath(&url, &format!("{ARCHIVE}*{SUFFIX}{EXT}"))?
		.iter()
		.filter_map(pending_of)
		.filter(|pending| pending.reach > reached)
		.collect();

	found.sort_by_key(|pending| pending.reach);

	Ok(found)
}

fn pending_of(entry: &rsync::Entry) -> Option<Pending> {
	let (name, archive) = entry.name.split_once('/')?;

	Some(Pending {
		reach: stamp::published(name, listen::PREFIX, SUFFIX)?.reach,
		name: name.to_string(),
		archive: archive.to_string(),
		size: entry.size,
	})
}

pub(super) fn room(root: &Path, pending: &[&Pending], at_once: u64) -> hmerr::Result<()> {
	let biggest = pending
		.iter()
		.map(|pending| pending.size)
		.max()
		.unwrap_or_default();

	space::require(root, biggest.saturating_mul(at_once))
}

pub(super) fn pull(
	root: &Path,
	pending: &Pending,
	downloading: &ProgressBar,
	verifying: &ProgressBar,
) -> hmerr::Result<()> {
	let url = url(&pending.name);

	rsync::pull(
		&format!("{url}{archive}", archive = pending.archive),
		&root.join(&pending.archive),
		downloading,
	)?;
	rsync::checked(&url, root, &rsync::checksum(&pending.archive))?;
	verifying.inc(1);

	Ok(())
}

pub(super) fn opened(
	root: &Path,
	pending: &Pending,
	bar: &ProgressBar,
) -> hmerr::Result<Incremental> {
	let dir = unpack(root, pending, bar)?;
	keep::discard(&root.join(&pending.archive))?;

	read(&dir, &pending.name)
}

pub(super) fn release(incremental: &Incremental) -> hmerr::Result<()> {
	keep::discard(&incremental.dir)
}

fn url(name: &str) -> String {
	format!("{host}/{MODULE}/{name}/", host = rsync::HOST)
}

fn unpack(root: &Path, pending: &Pending, bar: &ProgressBar) -> hmerr::Result<PathBuf> {
	let tar = root.join(&pending.archive);
	let file = fs::File::open(&tar).map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	tar::Archive::new(bar.wrap_read(file))
		.unpack(root)
		.map_err(|e| ioe!(tar.to_string_lossy(), e))?;

	let inner = root.join(stem(&pending.archive));
	if !inner.is_dir() {
		return Err(shapeless(&pending.archive).into());
	}

	let dir = root.join(UNPACKED);
	if dir.exists() {
		fs::remove_dir_all(&dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;
	}
	fs::rename(&inner, &dir).map_err(|e| ioe!(dir.to_string_lossy(), e))?;

	Ok(dir)
}

fn stem(archive: &str) -> &str {
	archive.strip_suffix(EXT).unwrap_or(archive)
}

fn read(dir: &Path, name: &str) -> hmerr::Result<Incremental> {
	let sequence = marker(dir, SEQUENCE)?;

	if sequence != SEQUENCE_UNDERSTOOD {
		return Err(unreadable(name, &sequence).into());
	}

	Ok(Incremental {
		start: marker(dir, START)?,
		end: marker(dir, END)?,
		name: name.to_string(),
		dir: dir.to_path_buf(),
	})
}

fn marker(dir: &Path, of: &str) -> hmerr::Result<String> {
	let path = dir.join(of);
	let read = fs::read_to_string(&path).map_err(|e| ioe!(path.to_string_lossy(), e))?;

	Ok(read.trim().to_string())
}

pub(super) fn weight(pending: &[&Pending]) -> u64 {
	pending.iter().map(|pending| pending.size).sum()
}

fn shapeless(archive: &str) -> GenericError {
	ge!(format!("{R}{B}{archive}{D}{R} held no dump directory{D}"))
}

fn unreadable(name: &str, sequence: &str) -> GenericError {
	ge!(
		format!(
			"{R}{B}{name}{D}{R} is written to schema {B}{sequence}{D}{R}, \
			this reads schema {B}{SEQUENCE_UNDERSTOOD}{D}"
		),
		h: "rebuild the index from a full dump of the same schema"
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn entry(name: &str, size: u64) -> rsync::Entry {
		rsync::Entry {
			name: name.to_string(),
			size,
		}
	}

	fn listed(name: &str) -> Vec<rsync::Entry> {
		vec![entry(
			&format!(
				"{name}/{ARCHIVE}{stem}{SUFFIX}{EXT}",
				stem = "2636-20260822-000002"
			),
			1 << 20,
		)]
	}

	#[test]
	fn a_listed_archive_yields_the_dump_it_belongs_to() {
		let pending = pending_of(&listed("listenbrainz-dump-2636-20260822-000002-incremental")[0]);

		assert_eq!(
			pending.as_ref().map(|pending| pending.name.as_str()),
			Some("listenbrainz-dump-2636-20260822-000002-incremental")
		);
		assert_eq!(
			pending.as_ref().map(|pending| pending.archive.as_str()),
			Some("listenbrainz-spark-dump-2636-20260822-000002-incremental.tar")
		);
		assert_eq!(
			pending.map(|pending| pending.reach),
			Some(20_260_822_000_002)
		);
	}

	#[test]
	fn an_archive_outside_a_dump_directory_is_not_pending() {
		assert!(pending_of(&entry("LATEST", 1)).is_none());
	}

	#[test]
	fn what_the_chain_weighs_is_the_sum_of_its_archives() {
		let pending: Vec<Pending> = ["2634-20260820-000003", "2635-20260821-000003"]
			.iter()
			.enumerate()
			.filter_map(|(step, stem)| {
				pending_of(&entry(
					&format!(
						"{prefix}{stem}{SUFFIX}/{ARCHIVE}{stem}{SUFFIX}{EXT}",
						prefix = listen::PREFIX
					),
					1 << (20 + step),
				))
			})
			.collect();

		assert_eq!(pending.len(), 2);
		assert_eq!(
			weight(&pending.iter().collect::<Vec<_>>()),
			(1 << 20) + (1 << 21)
		);
	}

	#[test]
	fn the_directory_inside_an_archive_is_named_after_it() {
		assert_eq!(
			stem("listenbrainz-spark-dump-2636-20260822-000002-incremental.tar"),
			"listenbrainz-spark-dump-2636-20260822-000002-incremental"
		);
	}

	#[test]
	fn a_dump_written_to_another_schema_is_refused() {
		let dir = crate::scratch::of("incremental", "schema");
		let _ = fs::write(dir.join(SEQUENCE), b"2");

		let said = read(&dir, "listenbrainz-dump-2636-20260822-000002-incremental")
			.err()
			.map(|e| format!("{e}"))
			.unwrap_or_default();

		assert!(said.contains("schema"), "{said}");
		let _ = fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_dump_of_the_schema_this_reads_yields_the_window_it_covers() {
		let dir = crate::scratch::of("incremental", "window");
		let _ = fs::write(dir.join(SEQUENCE), b"1\n");
		let _ = fs::write(dir.join(START), b"2026-08-21 00:00:03.155180+00:00\n");
		let _ = fs::write(dir.join(END), b"2026-08-22 00:00:02.641933+00:00\n");

		let read = read(&dir, "listenbrainz-dump-2636-20260822-000002-incremental");

		assert_eq!(
			read.as_ref().map(|read| read.start.as_str()).ok(),
			Some("2026-08-21 00:00:03.155180+00:00")
		);
		assert_eq!(
			read.map(|read| read.end).ok(),
			Some("2026-08-22 00:00:02.641933+00:00".to_string())
		);
		let _ = fs::remove_dir_all(&dir);
	}
}

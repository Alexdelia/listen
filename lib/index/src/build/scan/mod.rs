mod lane;
mod size;

use std::{
	fs,
	path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, WrapErr};

use super::{
	super::{
		board::{Board, Running},
		index::{self, layout::BUCKET},
		part, query, shard,
	},
	stage::{self, Stage},
};

use lane::Lane;
use size::Size;

pub(super) use lane::Held;

struct Unit {
	into: PathBuf,
	select: String,
}

pub(super) struct Scan {
	pub work: PathBuf,
	pub shard: Vec<String>,
	lane: Lane,
	board: Board<Stage>,
	batch: usize,
}

impl Scan {
	pub(super) fn of(work: &Path, dump: &Path) -> miette::Result<Self> {
		let shard = shard::of(dump)?;
		let db = duckdb::Connection::open_in_memory().into_diagnostic()?;
		let size = Size::of(size::offered(&db), shard.bytes, shard.path.len());

		db.execute_batch(&format!(
			r"
set memory_limit='{memory}';
set threads={thread};
set temp_directory='{work}/spill';
set preserve_insertion_order=false;
",
			memory = size.limit(),
			thread = size.thread,
			work = work.display()
		))
		.into_diagnostic()?;

		size.tell();

		Ok(Self {
			lane: Lane::of(db, size.lane)?,
			board: Board::of(&stage::PLAN, &size.batch)?,
			work: work.to_path_buf(),
			shard: shard.path,
			batch: size.batch,
		})
	}

	pub(super) fn take(&self) -> Held<'_> {
		self.lane.take()
	}

	pub(super) fn count(&self, of: &Path) -> miette::Result<u64> {
		query::count(&self.take(), of)
	}

	pub(super) fn stage(&self, stage: Stage) -> miette::Result<Running> {
		self.board.start(stage)
	}

	pub(super) fn step(&self, stage: Stage, into: &Path, select: &str) -> miette::Result<()> {
		part::step(&self.take(), &self.board, stage, into, select)
	}

	pub(super) fn batched(
		&self,
		stage: Stage,
		part: &str,
		query: &dyn Fn(&str) -> String,
	) -> miette::Result<PathBuf> {
		let partial = self.work.join(part);
		fs::create_dir_all(&partial)
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot create {}", partial.display()))?;

		let per = self.shard.len().div_ceil(self.batch);
		let unit: Vec<Unit> = self
			.shard
			.chunks(per.max(1))
			.enumerate()
			.map(|(step, chunk)| Unit {
				into: partial.join(format!("{step}.parquet")),
				select: query(&shard::quoted(chunk)),
			})
			.collect();

		self.produce(stage, &unit)?;

		Ok(partial)
	}

	pub(super) fn bucketed(
		&self,
		into: &Path,
		stage: Stage,
		query: &dyn Fn(u32) -> String,
	) -> miette::Result<()> {
		fs::create_dir_all(into)
			.into_diagnostic()
			.wrap_err_with(|| format!("cannot create {}", into.display()))?;

		let unit: Vec<Unit> = (0..BUCKET)
			.map(|bucket| Unit {
				into: into.join(index::layout::shard(bucket)),
				select: query(bucket),
			})
			.collect();

		self.produce(stage, &unit)
	}

	fn produce(&self, stage: Stage, unit: &[Unit]) -> miette::Result<()> {
		let bar = self.stage(stage)?;
		bar.set_length(unit.len() as u64);

		self.lane.spread(unit, &bar, |db, unit| {
			if query::done(db, &unit.into) {
				return Ok(());
			}

			query::copy(db, &unit.into, &unit.select)
		})
	}
}

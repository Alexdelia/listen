mod channel;
mod download_order;
mod fetch;
mod filter;
mod gain;
mod playlist;
mod progress;
mod rate;
mod refresh;
mod remove;
mod report;
mod tag;

use std::{
	path::{Path, PathBuf},
	thread,
};

use async_std::{channel::Sender, task::block_on};

use crate::{
	declaration::{Source, parse},
	env, library,
	prefetch::Prefetch,
	prompt,
};

use channel::{Action, Status};
use filter::{GroupedEntry, SyncEntry};
use progress::Count;

pub(crate) fn run(path: &Path, refresh_metadata: bool) -> miette::Result<()> {
	if refresh_metadata {
		gain::ready()?;
		let list = parse::parse(path)?;
		block_on(refresh::metadata::run(&list))?;
		return refresh::gain::run(&list);
	}

	env::load()?;

	let list = parse::parse(path)?;

	if let Err(e) = Prefetch::cached().and_then(|prefetch| prefetch.prune()) {
		eprintln!("{e:?}");
	}

	let pending = rate::pending(&list)?;

	let filter::Filtered { entry: sync, kept } = filter::sync(list)?;

	let remove = report::report(&sync);

	if remove {
		let yes = prompt::confirm("proceed with update", true)?;

		if !yes {
			return Ok(());
		}
	}

	let ungained = match gain::ready() {
		Ok(()) => Some(ungained(kept)),
		Err(e) => {
			eprintln!("{:?}", e.wrap_err("no replaygain is written this run"));
			None
		}
	};

	let (tx, rx) = async_std::channel::unbounded::<Status>();

	let rating = acquire_rating(pending, &tx);

	let total = Count {
		fetch: sync.fs.add.len(),
		gain: ungained
			.as_ref()
			.map_or(0, |ungained| sync.fs.add.len() + ungained.len()),
		remove: sync.fs.remove.len(),
		playlist: sync.q.len() + sync.playlist.len(),
		rating: rating.count(),
	};

	process(sync, ungained, rating.submit(), tx);
	println!();
	progress::render(total, &rx)
}

enum Rating {
	Submit(String, rate::Pending),
	Failed(usize),
	Skip,
}

impl Rating {
	const fn count(&self) -> usize {
		match self {
			Self::Submit(_, pending) => pending.rating.len(),
			Self::Failed(count) => *count,
			Self::Skip => 0,
		}
	}

	fn submit(self) -> Option<(String, rate::Pending)> {
		match self {
			Self::Submit(bearer, pending) => Some((bearer, pending)),
			_ => None,
		}
	}
}

fn acquire_rating(pending: Option<rate::Pending>, tx: &Sender<Status>) -> Rating {
	let Some(pending) = pending else {
		return Rating::Skip;
	};

	if pending.rating.is_empty() {
		return Rating::Skip;
	}

	match rate::acquire(&pending) {
		Ok(Some(bearer)) => Rating::Submit(bearer, pending),
		Ok(None) => Rating::Skip,
		Err(e) => {
			block_on(channel::report(tx, Action::SubmitRating(0), Err(e)));
			Rating::Failed(pending.rating.len())
		}
	}
}

fn ungained(kept: Vec<Source>) -> Vec<PathBuf> {
	kept.into_iter()
		.map(library::recording::path)
		.filter(|path| !library::gain::carried(path))
		.collect()
}

fn process(
	sync: GroupedEntry<SyncEntry>,
	ungained: Option<Vec<PathBuf>>,
	rating: Option<(String, rate::Pending)>,
	tx: Sender<Status>,
) {
	if let Some((bearer, pending)) = rating {
		detach(&tx, |tx| block_on(rate::sync(bearer, pending, tx)));
	}

	let GroupedEntry { fs, q, playlist } = sync;
	let SyncEntry { add, remove } = fs;

	detach(&tx, move |tx| {
		let gain_ready = ungained.is_some();
		thread::scope(|scope| {
			scope.spawn(|| {
				gain::each(ungained.unwrap_or_default(), |status| {
					block_on(channel::report(&tx, Action::AddGain, status));
				});
			});
			block_on(fetch::fetch(&add, gain_ready, tx.clone()));
		});
		block_on(playlist::all(q, playlist, tx));
	});
	detach(&tx, move |tx| {
		block_on(remove::remove(&remove, tx));
	});

	drop(tx);
}

fn detach(tx: &Sender<Status>, work: impl FnOnce(Sender<Status>) + Send + 'static) {
	let tx = tx.clone();

	thread::spawn(move || work(tx));
}

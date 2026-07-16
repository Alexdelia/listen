mod channel;
mod fetch;
mod filter;
mod playlist;
mod progress;
mod rate;
mod refresh;
mod remove;
mod report;
mod tag;

use std::{future::IntoFuture, path::Path, thread};

use async_std::{channel::Sender, task::block_on};
use hmerr::ioe;

use crate::{declaration::parse, env};

use channel::{Action, Status};
use filter::{GroupedEntry, SyncEntry};
use progress::Count;

pub fn run(path: &Path, refresh_metadata: bool) -> hmerr::Result<()> {
	if refresh_metadata {
		let list = parse::parse(path)?;
		return block_on(refresh::metadata::run(&list));
	}

	env::load()?;

	let list = parse::parse(path)?;

	let pending = rate::pending(&list)?;

	let sync = filter::sync(list)?;

	let remove = report::report(&sync);

	if remove {
		let yes = ux::ask_yn("do you want to proceed with this update?", true)
			.map_err(|e| ioe!("stdin", e))?;

		if !yes {
			return Ok(());
		}
	}

	let (tx, rx) = async_std::channel::unbounded::<Status>();

	let rating = acquire_rating(pending, &tx);

	let total = Count {
		fetch: sync.fs.add.len(),
		remove: sync.fs.remove.len(),
		playlist: sync.q.len() + sync.playlist.len(),
		rating: rating.count(),
	};

	process(sync, rating.submit(), tx);
	println!();
	progress::render(total, &rx)
}

enum Rating {
	Submit(String, rate::Pending),
	Failed(usize),
	Skip,
}

impl Rating {
	fn count(&self) -> usize {
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
			block_on(channel::report(
				tx,
				Status {
					action: Action::SubmitRating(0),
					status: Err(e.to_string()),
				},
			));
			Rating::Failed(pending.rating.len())
		}
	}
}

fn process(
	sync: GroupedEntry<SyncEntry>,
	rating: Option<(String, rate::Pending)>,
	tx: Sender<Status>,
) {
	if let Some((bearer, pending)) = rating {
		let txc = tx.clone();
		thread::spawn(move || {
			block_on(rate::sync(bearer, pending, txc).into_future());
		});
	}

	let txc = tx.clone();
	thread::spawn(move || {
		block_on(fetch::fetch(&sync.fs.add, txc).into_future());
	});
	let txc = tx.clone();
	thread::spawn(move || {
		block_on(remove::remove(&sync.fs.remove, txc).into_future());
	});

	for (q, sync) in sync.q {
		let txc = tx.clone();
		thread::spawn(move || {
			block_on(playlist::q(q, sync, txc).into_future());
		});
	}
	for (playlist, sync) in sync.playlist {
		let txc = tx.clone();
		thread::spawn(move || {
			block_on(playlist::playlist(playlist, sync, txc).into_future());
		});
	}

	drop(tx);
}

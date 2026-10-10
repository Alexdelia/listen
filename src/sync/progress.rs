use async_std::channel::Receiver;
use indicatif::{MultiProgress, ProgressBar};

use crate::bar::template;

use super::channel::{Action, Status};

#[derive(Default, Clone, Copy)]
pub(super) struct Count {
	pub fetch: usize,
	pub remove: usize,
	pub playlist: usize,
	pub rating: usize,
}

pub(super) fn render(total: Count, rx: &Receiver<Status>) -> hmerr::Result<()> {
	let mp = MultiProgress::new();

	let pb_playlist = bar(&mp, total.playlist, "playlist", "magenta")?;
	let pb_rating = bar(&mp, total.rating, "rating", "yellow")?;
	let pb_remove = bar(&mp, total.remove, "remove", "red")?;
	let pb_fetch = bar(&mp, total.fetch, "fetch", "blue")?;
	let pb_download = bar(&mp, total.fetch, "download", "cyan")?;
	let pb_metadata = bar(&mp, total.fetch, "metadata", "green")?;

	let mut err = vec![];

	while let Ok(status) = rx.recv_blocking() {
		match status.action {
			Action::FetchMusicBrainz => {
				pb_fetch.inc(1);
				pb_download.tick();
				pb_metadata.tick();
			}
			Action::FetchStreaming => {
				pb_fetch.tick();
				pb_download.inc(1);
				pb_metadata.tick();
			}
			Action::AddMetadata => {
				pb_fetch.tick();
				pb_download.tick();
				pb_metadata.inc(1);
			}
			Action::RemoveFile => pb_remove.inc(1),
			Action::SyncPlaylist => pb_playlist.inc(1),
			Action::ReadTag => pb_playlist.tick(),
			Action::SubmitRating(count) => pb_rating.inc(count as u64),
		}

		if let Err(e) = status.status {
			mp.suspend(|| eprintln!("{e}\n"));
			err.push(e);
		}
	}

	finished(&pb_fetch);
	finished(&pb_download);
	finished(&pb_metadata);
	finished(&pb_remove);
	finished(&pb_playlist);
	finished(&pb_rating);

	if !err.is_empty() {
		eprint!("\n\nerrors:\n\n");
		for e in err {
			eprintln!("{e}");
		}
		eprint!("\n\n\n");
	}

	Ok(())
}

fn bar(mp: &MultiProgress, total: usize, title: &str, color: &str) -> hmerr::Result<ProgressBar> {
	let bar = mp.add(ProgressBar::new(total as u64));
	bar.set_style(template(title, color)?);

	if total > 0 {
		bar.tick();
	}

	Ok(bar)
}

fn finished(bar: &ProgressBar) {
	if bar.length().is_some_and(|total| total > 0) {
		bar.finish();
	}
}

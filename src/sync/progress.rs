use async_std::channel::Receiver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

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
			Action::SubmitRating(count) => pb_rating.inc(count as u64),
		}

		if let Err(e) = status.status {
			eprintln!("{e}\n");
			err.push(e);
		}
	}

	finished(&pb_fetch, total.fetch);
	finished(&pb_download, total.fetch);
	finished(&pb_metadata, total.fetch);
	finished(&pb_remove, total.remove);
	finished(&pb_playlist, total.playlist);
	finished(&pb_rating, total.rating);

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

fn finished(bar: &ProgressBar, total: usize) {
	if total > 0 {
		bar.finish();
	}
}

pub(super) fn template(title: &str, color: &str) -> hmerr::Result<ProgressStyle> {
	let title = format!("{title:>8}");

	ProgressStyle::with_template(
		&[
			&title,
			" {wide_bar:.",
			color,
			"/white} {pos:>4.bold.green}/{len:4.bold} {percent:>3.bold.green}% {elapsed:>3.bold.blue}|{eta:3.bold.magenta}",
		]
		.join(""),
	)
	.map_err(|e| format!("failed to create progress style\n{e}").into())
}

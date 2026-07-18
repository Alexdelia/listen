use async_std::channel::Receiver;
use indicatif::{MultiProgress, ProgressStyle};

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

	let template = |title: &str, color: &str| -> hmerr::Result<ProgressStyle> {
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
	};

	let pb_playlist = mp.add(indicatif::ProgressBar::new(total.playlist as u64));
	pb_playlist.set_style(template("playlist", "magenta")?);
	if total.playlist > 0 {
		pb_playlist.tick();
	}

	let pb_rating = mp.add(indicatif::ProgressBar::new(total.rating as u64));
	pb_rating.set_style(template("rating", "yellow")?);
	if total.rating > 0 {
		pb_rating.tick();
	}

	let pb_remove = mp.add(indicatif::ProgressBar::new(total.remove as u64));
	pb_remove.set_style(template("remove", "red")?);
	if total.remove > 0 {
		pb_remove.tick();
	}

	let pb_fetch = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_fetch.set_style(template("fetch", "blue")?);
	let pb_download = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_download.set_style(template("download", "cyan")?);
	let pb_metadata = mp.add(indicatif::ProgressBar::new(total.fetch as u64));
	pb_metadata.set_style(template("metadata", "green")?);
	if total.fetch > 0 {
		pb_fetch.tick();
		pb_download.tick();
		pb_metadata.tick();
	}

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

	if total.fetch > 0 {
		pb_fetch.finish();
		pb_download.finish();
		pb_metadata.finish();
	}
	if total.remove > 0 {
		pb_remove.finish();
	}
	if total.playlist > 0 {
		pb_playlist.finish();
	}
	if total.rating > 0 {
		pb_rating.finish();
	}

	if !err.is_empty() {
		eprint!("\n\nerrors:\n\n");
		for e in err {
			eprintln!("{e}");
		}
		eprint!("\n\n\n");
	}

	Ok(())
}

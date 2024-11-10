use std::process::Command;

use crate::{
	entry::{Entry, Source},
	env,
};

pub fn fetch(entry: &Entry) {
	match &entry.s {
		Source::Yt(id) => {
			println!("fetching yt {id}");
		}
		Source::Sc(user, slug) => soundcloud(user, slug),
	}
}

const SOUNDCLOUD_URL: &str = "https://soundcloud.com";
const SOUNDCLOUD_DOWNLOADER: &str = "scdl";

fn soundcloud(user: &str, slug: &str) {
	match Command::new(SOUNDCLOUD_DOWNLOADER)
		.args(&[
			"--client-id",
			&env::get(env::Var::SoundcloudClientId).expect("SOUNDCLOUD_CLIENT_ID not set"),
			"--onlymp3",
			"--extract-artist",
			"-l",
			&format!("{SOUNDCLOUD_URL}/{user}/{slug}"),
		])
		.output()
	{
		Err(e) => {
			dbg!(e);
		}
		Ok(output) => {
			dbg!(output);
		}
	}
}

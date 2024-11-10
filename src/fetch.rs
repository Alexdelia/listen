use crate::entry::{Entry, Source};

pub fn fetch(entry: &Entry) {
	match &entry.s {
		Source::Yt(id) => {
			println!("fetching yt {}", id);
		}
		Source::Sc(user, id) => soundcloud(user, id),
	}
}

fn soundcloud(user: &str, id: &str) {
	println!("fetching sc {} {}", user, id);
}

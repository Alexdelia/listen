use std::{cmp::Reverse, collections::HashSet};

use serde::Serialize;

use crate::declaration::Source;

use super::super::fetch::ListenCount;

#[derive(Serialize)]
pub(crate) struct Undeclared {
	pub mbid: Source,
	pub listen: u32,
	pub track: String,
	pub artist: String,
}

pub(super) fn undeclared(listen: &ListenCount, consumed: &HashSet<&Source>) -> Vec<Undeclared> {
	let mut undeclared = listen
		.iter()
		.filter(|(mbid, _)| !consumed.contains(mbid))
		.map(|(mbid, l)| Undeclared {
			mbid: *mbid,
			listen: l.count,
			track: l.track.clone(),
			artist: l.artist.clone(),
		})
		.collect::<Vec<_>>();

	undeclared.sort_by_key(|undeclared| Reverse(undeclared.listen));

	undeclared
}

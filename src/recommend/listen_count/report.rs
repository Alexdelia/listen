use serde::Serialize;

use crate::args::RecommendSort;

use super::render;

#[derive(Serialize)]
pub(in crate::recommend) struct Report {
	pub artist: String,
	pub sort: RecommendSort,
	pub recording: usize,
	pub dated: usize,
	pub ranked: usize,
}

impl Report {
	pub(in crate::recommend) fn print(&self) {
		println!("{}", render::header(self));
	}
}

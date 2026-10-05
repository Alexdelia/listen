use listen_index::Meta;
use serde::Serialize;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use super::{
	super::{feed, skip::Count, stream::Skipped},
	collect::Pick,
};

#[derive(Serialize)]
pub(in crate::recommend) struct Arg<'a> {
	pub target: Option<&'a str>,
	pub source: RecommendSource,
	pub sort: RecommendSort,
	pub unlistened: bool,
	pub limit: usize,
	#[serde(flatten)]
	pub island: &'a IslandArg,
}

#[derive(Serialize)]
pub(in crate::recommend) struct Document<'a> {
	pub arg: Arg<'a>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub index: Option<&'a Meta>,
	pub skip: Count,
	pub feed: Vec<feed::Report>,
	pub recommendation: Vec<Pick>,
	pub skipped: &'a [Skipped],
}

#[cfg(test)]
mod tests {
	use super::*;

	fn island() -> IslandArg {
		IslandArg {
			popularity_damp: 0.5,
			granularity: 1.5,
			island: None,
			ask: false,
			seed: Vec::new(),
			genre: vec!["touhou".to_string()],
			allow_known_artist: false,
			backtest: false,
		}
	}

	fn document(island: &IslandArg) -> serde_json::Value {
		serde_json::to_value(Document {
			arg: Arg {
				target: None,
				source: RecommendSource::Island,
				sort: RecommendSort::Popularity,
				unlistened: false,
				limit: 10,
				island,
			},
			index: None,
			skip: Count {
				declared: 3,
				declined: 1,
			},
			feed: Vec::new(),
			recommendation: Vec::new(),
			skipped: &[],
		})
		.unwrap_or_default()
	}

	#[test]
	fn a_document_carries_every_top_level_section() {
		let value = document(&island());

		for key in ["arg", "skip", "feed", "recommendation", "skipped"] {
			assert!(value.get(key).is_some(), "{key} missing in {value}");
		}
	}

	#[test]
	fn a_run_without_a_local_index_leaves_the_index_out() {
		assert!(document(&island()).get("index").is_none());
	}

	#[test]
	fn the_island_args_sit_flat_beside_the_others() {
		let value = document(&island());

		assert_eq!(value["arg"]["source"], "island");
		assert_eq!(value["arg"]["genre"][0], "touhou");
		assert_eq!(value["arg"]["limit"], 10);
	}
}

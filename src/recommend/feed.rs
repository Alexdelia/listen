use chrono::NaiveDate;
use serde::Serialize;

use super::{forecast, island, listen_count, recommendation::Recommendation, skip::Skip};

pub(super) trait Feed {
	fn next(&mut self, skip: &Skip) -> hmerr::Result<Option<Recommendation>>;
}

pub(super) struct Fed {
	pub feed: Box<dyn Feed>,
	pub report: Report,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Report {
	Island(island::Report),
	Forecast(forecast::Report),
	CollaborativeFiltering {
		username: String,
	},
	WeeklyExploration {
		username: String,
		week: Vec<NaiveDate>,
	},
	ListenCount(listen_count::Report),
}

impl Report {
	pub(super) fn print(&self) {
		match self {
			Self::Island(report) => report.print(),
			Self::Forecast(report) => report.print(),
			Self::ListenCount(report) => report.print(),
			Self::CollaborativeFiltering { .. } | Self::WeeklyExploration { .. } => {}
		}
	}
}

#[cfg(test)]
pub(super) struct Canned(pub std::collections::VecDeque<Recommendation>);

#[cfg(test)]
impl Feed for Canned {
	fn next(&mut self, _skip: &Skip) -> hmerr::Result<Option<Recommendation>> {
		Ok(self.0.pop_front())
	}
}

#[cfg(test)]
pub(super) fn canned(recommendation: Vec<Recommendation>) -> Box<dyn Feed> {
	Box::new(Canned(recommendation.into()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_remote_report_serializes_under_its_source_name() {
		let value = serde_json::to_value(Report::CollaborativeFiltering {
			username: "alexdelia".to_string(),
		})
		.unwrap_or_default();

		assert_eq!(value["collaborative_filtering"]["username"], "alexdelia");
	}
}

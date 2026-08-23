use serde::{Deserialize, Serialize};

use crate::recommend::island::index::own::{self, Gap};

use super::super::{
	age,
	fetch::{Listen, ListenCount},
	gap,
};

#[derive(Deserialize, Serialize)]
pub(crate) struct Held {
	pub dump: String,
	#[serde(default)]
	pub reached: String,
	#[serde(default)]
	pub gap: Vec<Gap>,
	pub covered: i64,
	pub count: ListenCount,
	#[serde(default)]
	pub fold: Option<ListenCount>,
}

pub(super) struct Carried {
	pub reached: String,
	pub gap: Vec<Gap>,
	pub covered: i64,
	pub fold: ListenCount,
}

impl Held {
	pub(crate) fn ago(&self) -> hmerr::Result<u64> {
		age::days_since(self.reached_at())
	}

	pub(crate) fn counted(&self) -> ListenCount {
		let mut count = self.count.clone();

		for (mbid, folded) in self.fold.iter().flatten() {
			let listen = count.entry(*mbid).or_insert_with(|| Listen {
				count: 0,
				track: folded.track.clone(),
				artist: folded.artist.clone(),
			});

			listen.count = listen.count.saturating_add(folded.count);
		}

		count
	}

	pub(super) fn reach(&self) -> &str {
		if self.reached.is_empty() {
			return &self.dump;
		}

		&self.reached
	}

	pub(super) fn reached_at(&self) -> i64 {
		gap::seconds(self.reach()).unwrap_or(self.covered)
	}

	pub(super) fn foldable(&self) -> bool {
		own::stamped(self.reach())
	}

	pub(super) fn apart(&self) -> bool {
		self.fold.is_some() || self.reach() == self.dump
	}

	pub(super) fn carried(self) -> Carried {
		Carried {
			reached: self.reach().to_string(),
			gap: self.gap,
			covered: self.covered,
			fold: self.fold.unwrap_or_default(),
		}
	}
}

impl Carried {
	pub(super) fn of(dump: &str) -> Self {
		Self {
			reached: dump.to_string(),
			gap: Vec::new(),
			covered: 0,
			fold: ListenCount::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{
		super::fixture::{DUMP, LATEST, NEWER, held},
		*,
	};

	#[test]
	fn counts_read_before_an_incremental_was_folded_carry_on_from_the_dump_they_came_from() {
		assert_eq!(held().reach(), DUMP);

		let folded = Held {
			reached: NEWER.to_string(),
			..held()
		};

		assert_eq!(folded.reach(), NEWER);
	}

	#[test]
	fn how_far_the_counts_reach_is_the_dump_they_stop_at_not_the_last_listen_they_hold() {
		let quiet = Held {
			reached: LATEST.to_string(),
			..held()
		};

		assert_eq!(quiet.reached_at(), 1_787_356_802);
	}

	#[test]
	fn counts_stopping_at_a_stamp_that_cannot_be_read_fall_back_to_the_last_listen_they_hold() {
		let unreadable = Held {
			reached: "listen".to_string(),
			..held()
		};

		assert_eq!(unreadable.reached_at(), 1_783_814_404);
	}
}

use std::collections::VecDeque;

use crate::recommend::{feed::Feed, recommendation::Recommendation, skip::Skip};

use super::payload::Page;

pub(super) struct Cursor<F> {
	fetch: F,
	offset: usize,
	drained: bool,
	buffer: VecDeque<Recommendation>,
}

impl<F> Cursor<F>
where
	F: FnMut(usize) -> hmerr::Result<Page>,
{
	pub(super) fn new(fetch: F) -> Self {
		Self {
			fetch,
			offset: 0,
			drained: false,
			buffer: VecDeque::new(),
		}
	}
}

impl<F> Feed for Cursor<F>
where
	F: FnMut(usize) -> hmerr::Result<Page>,
{
	fn next(&mut self, _skip: &Skip) -> hmerr::Result<Option<Recommendation>> {
		if let Some(recommendation) = self.buffer.pop_front() {
			return Ok(Some(recommendation));
		}

		if self.drained {
			return Ok(None);
		}

		let page = (self.fetch)(self.offset)?;

		self.offset += page.fetched;
		self.drained = page.fetched == 0 || self.offset >= page.total;
		self.buffer.extend(page.recommendation);

		Ok(self.buffer.pop_front())
	}
}

#[cfg(test)]
mod tests {
	use std::{cell::RefCell, rc::Rc};

	use super::*;
	use crate::{declaration::Source, recommend::recommendation::Origin};

	fn recommendation(nibble: u8) -> Recommendation {
		Recommendation {
			mbid: Source::from_bytes([nibble; 16]),
			origin: Origin::CollaborativeFiltering {
				position: nibble.into(),
				score: 1.0,
				latest_listened_at: None,
			},
		}
	}

	fn page(nibble: &[u8], total: usize) -> Page {
		Page {
			recommendation: nibble.iter().copied().map(recommendation).collect(),
			fetched: nibble.len(),
			total,
		}
	}

	fn drain<F>(cursor: &mut Cursor<F>) -> Vec<u8>
	where
		F: FnMut(usize) -> hmerr::Result<Page>,
	{
		let mut seen = Vec::new();

		while let Ok(Some(recommendation)) = cursor.next(&Skip::default()) {
			seen.push(recommendation.mbid.as_bytes()[0]);
		}

		seen
	}

	#[test]
	fn a_drained_buffer_asks_for_the_next_page() {
		let asked = Rc::new(RefCell::new(Vec::new()));
		let recorded = Rc::clone(&asked);

		let mut cursor = Cursor::new(move |offset| {
			recorded.borrow_mut().push(offset);

			Ok(match offset {
				0 => page(&[1, 2], 4),
				_ => page(&[3, 4], 4),
			})
		});

		assert_eq!(drain(&mut cursor), vec![1, 2, 3, 4]);
		assert_eq!(*asked.borrow(), vec![0, 2]);
	}

	#[test]
	fn the_total_stops_the_paging() {
		let asked = Rc::new(RefCell::new(Vec::new()));
		let recorded = Rc::clone(&asked);

		let mut cursor = Cursor::new(move |offset| {
			recorded.borrow_mut().push(offset);

			Ok(page(&[1, 2], 2))
		});

		assert_eq!(drain(&mut cursor), vec![1, 2]);
		assert_eq!(*asked.borrow(), vec![0]);
	}

	#[test]
	fn an_empty_page_stops_the_paging() {
		let asked = Rc::new(RefCell::new(Vec::new()));
		let recorded = Rc::clone(&asked);

		let mut cursor = Cursor::new(move |offset| {
			recorded.borrow_mut().push(offset);

			Ok(page(&[], 100))
		});

		assert!(drain(&mut cursor).is_empty());
		assert_eq!(*asked.borrow(), vec![0]);
	}
}

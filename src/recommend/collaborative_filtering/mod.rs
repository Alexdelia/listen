mod cursor;
mod fetch;
mod payload;

use cursor::Cursor;

use super::feed::Feed;

pub(super) fn feed(username: String) -> impl Feed {
	Cursor::new(move |offset| payload::page(&fetch::recording(&username, offset)?, offset))
}

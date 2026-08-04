mod cursor;
mod page;

use cursor::Cursor;

use super::feed::Feed;

pub(super) fn feed(username: String) -> impl Feed {
	Cursor::new(move |offset| page::page(&username, offset))
}

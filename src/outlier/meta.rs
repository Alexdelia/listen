use std::collections::HashMap;

use crate::{
	declaration::{Entry, Source},
	library,
};

pub(super) type Meta = HashMap<Source, (String, String)>;

pub(super) fn declared(list: &[Entry]) -> Meta {
	list.iter()
		.filter_map(|entry| {
			library::tag::title_artist(entry.s).map(|title_artist| (entry.s, title_artist))
		})
		.collect()
}

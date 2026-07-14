use ansi::{abbrev::D, hex};

use crate::entry::Q;

pub const Q0: &str = hex!(#d1ba47);
pub const Q1: &str = hex!(#a1d147);
pub const Q2: &str = hex!(#47d160);
pub const Q3: &str = hex!(#8147d1);
pub const Q4: &str = hex!(#fc0380);

pub fn q(q: Q) -> &'static str {
	match q {
		0 => Q0,
		1 => Q1,
		2 => Q2,
		3 => Q3,
		4 => Q4,
		_ => D,
	}
}

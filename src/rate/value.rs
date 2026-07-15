use ansi::abbrev::{B, D, R};
use hmerr::ge;

use crate::entry::Q;

pub type Value = u8;

pub fn from_q(q: Q) -> hmerr::Result<Value> {
	match q {
		0 => Ok(20),  // 1
		1 => Ok(50),  // 2.5
		2 => Ok(70),  // 3.5
		3 => Ok(90),  // 4.5
		4 => Ok(100), // 5
		_ => Err(ge!(format!("{R}no rating defined for {B}q{q}{D}")).into()),
	}
}

use ansi::abbrev::{B, D, R};
use hmerr::ge;

use crate::entry::Q;

pub type Star = u8;

pub fn from_q(q: Q) -> hmerr::Result<Star> {
	match q {
		0 => Ok(1),
		1 => Ok(3),
		2 => Ok(4),
		3 | 4 => Ok(5),
		_ => Err(ge!(format!("{R}no star rating defined for {B}q{q}{D}")).into()),
	}
}

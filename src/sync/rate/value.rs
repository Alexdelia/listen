use crate::declaration::{Q, Q_MAX};

pub type Value = u8;

const VALUE: [Value; Q_MAX as usize + 1] = [
	20,  // 1
	50,  // 2.5
	70,  // 3.5
	90,  // 4.5
	100, // 5
];

pub fn from_q(q: Q) -> Value {
	VALUE[q as usize]
}

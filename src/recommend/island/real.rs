#[expect(
	clippy::cast_precision_loss,
	reason = "a count of seeds, users or genre tokens, orders of magnitude below the f32 mantissa"
)]
pub(super) const fn of(count: usize) -> f32 {
	count as f32
}

#[expect(
	clippy::cast_precision_loss,
	reason = "a count of listeners, orders of magnitude below the f64 mantissa"
)]
pub(super) const fn wide(count: usize) -> f64 {
	count as f64
}

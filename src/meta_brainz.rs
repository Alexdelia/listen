use std::{
	num::NonZeroU32,
	sync::{Arc, LazyLock},
};

use async_std::task::block_on;
use musicbrainz_rs::api_bindium::governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

#[allow(clippy::unwrap_used, reason = "1 is valid NonZeroU32")]
const PER_SECOND: NonZeroU32 = NonZeroU32::new(1).unwrap();

static LIMITER: LazyLock<Arc<DefaultDirectRateLimiter>> =
	LazyLock::new(|| Arc::new(RateLimiter::direct(Quota::per_second(PER_SECOND))));

pub(crate) fn limiter() -> Arc<DefaultDirectRateLimiter> {
	LIMITER.clone()
}

pub(crate) fn block_ready() {
	block_on(LIMITER.until_ready());
}

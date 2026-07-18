use std::{
	num::NonZeroU32,
	sync::{Arc, LazyLock},
};

use async_std::task::block_on;
use musicbrainz_rs::api_bindium::governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

#[allow(clippy::unwrap_used, reason = "1 is valid NonZeroU32")]
const PER_SECOND: NonZeroU32 = NonZeroU32::new(1).unwrap();
#[allow(clippy::unwrap_used, reason = "5 is valid NonZeroU32")]
const BURST: NonZeroU32 = NonZeroU32::new(5).unwrap();

static LIMITER: LazyLock<Arc<DefaultDirectRateLimiter>> = LazyLock::new(|| {
	Arc::new(RateLimiter::direct(
		Quota::per_second(PER_SECOND).allow_burst(BURST),
	))
});

pub fn limiter() -> Arc<DefaultDirectRateLimiter> {
	LIMITER.clone()
}

pub async fn ready() {
	LIMITER.until_ready().await;
}

pub fn block_ready() {
	block_on(LIMITER.until_ready());
}

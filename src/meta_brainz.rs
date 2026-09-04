use std::{
	num::NonZeroU32,
	sync::{Arc, LazyLock},
	thread,
	time::Duration,
};

use ansi::abbrev::{B, D};
use async_std::task::block_on;
use hmerr::ge;
use musicbrainz_rs::api_bindium::governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use ureq::{
	Body,
	http::{HeaderMap, Response, StatusCode},
};

#[allow(clippy::unwrap_used, reason = "1 is valid NonZeroU32")]
const PER_SECOND: NonZeroU32 = NonZeroU32::new(1).unwrap();

const RETRY: u8 = 4;

const UNSAID_WAIT: Duration = Duration::from_secs(2);
const LONGEST_WAIT: Duration = Duration::from_secs(60);

const RETRY_AFTER: &str = "retry-after";
const RESET_IN: &str = "x-ratelimit-reset-in";

static LIMITER: LazyLock<Arc<DefaultDirectRateLimiter>> =
	LazyLock::new(|| Arc::new(RateLimiter::direct(Quota::per_second(PER_SECOND))));

pub(crate) fn limiter() -> Arc<DefaultDirectRateLimiter> {
	LIMITER.clone()
}

pub(crate) fn block_ready() {
	block_on(LIMITER.until_ready());
}

pub(crate) struct Sent {
	pub status: StatusCode,
	pub body: String,
}

pub(crate) fn send(
	attempt: impl Fn() -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	let mut left = RETRY;

	loop {
		block_ready();

		let mut response = attempt().map_err(|e| ge!(format!("{failure}\n{e}")))?;
		let status = response.status();

		if let Some(wait) = throttling(&response) {
			if too_long(wait) {
				return Err(asked_for_longer(failure, wait));
			}

			if left > 0 {
				left -= 1;
				thread::sleep(wait);
				continue;
			}
		}

		return Ok(Sent {
			status,
			body: response
				.body_mut()
				.read_to_string()
				.map_err(|e| ge!(format!("{failure}\n{e}")))?,
		});
	}
}

fn throttling(response: &Response<Body>) -> Option<Duration> {
	throttled(response.status()).then(|| wait(said(response.headers())))
}

fn throttled(status: StatusCode) -> bool {
	status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE
}

fn said(headers: &HeaderMap) -> Option<&str> {
	headers
		.get(RETRY_AFTER)
		.or_else(|| headers.get(RESET_IN))?
		.to_str()
		.ok()
}

fn wait(said: Option<&str>) -> Duration {
	said.and_then(seconds).unwrap_or(UNSAID_WAIT)
}

fn too_long(wait: Duration) -> bool {
	wait > LONGEST_WAIT
}

fn asked_for_longer(failure: &str, wait: Duration) -> Box<dyn std::error::Error> {
	let seconds = wait.as_secs();

	ge!(
		format!("{failure}\nthrottled for {B}{seconds}s{D}, longer than this run waits out"),
		h: format!("the service is asking to be left alone, run it again in {seconds}s")
	)
	.into()
}

fn seconds(said: &str) -> Option<Duration> {
	Some(Duration::from_secs(said.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
	use ureq::http::HeaderValue;

	use super::*;

	#[test]
	fn the_two_statuses_that_mean_slow_down_are_the_only_ones_waited_out() {
		assert!(throttled(StatusCode::TOO_MANY_REQUESTS));
		assert!(throttled(StatusCode::SERVICE_UNAVAILABLE));

		assert!(!throttled(StatusCode::OK));
		assert!(!throttled(StatusCode::NO_CONTENT));
		assert!(!throttled(StatusCode::NOT_FOUND));
		assert!(!throttled(StatusCode::INTERNAL_SERVER_ERROR));
	}

	#[test]
	fn the_wait_a_service_names_in_seconds_is_the_wait_taken() {
		assert_eq!(wait(Some("30")), Duration::from_secs(30));
	}

	#[test]
	fn a_named_wait_is_read_through_the_spaces_around_it() {
		assert_eq!(wait(Some(" 7 ")), Duration::from_secs(7));
	}

	#[test]
	fn a_service_naming_no_wait_is_waited_out_the_unsaid_one() {
		assert_eq!(wait(None), UNSAID_WAIT);
	}

	#[test]
	fn a_retry_after_holding_a_date_rather_than_seconds_falls_back_to_the_unsaid_wait() {
		assert_eq!(wait(Some("Wed, 21 Oct 2015 07:28:00 GMT")), UNSAID_WAIT);
	}

	#[test]
	fn a_wait_longer_than_the_longest_one_is_kept_whole_rather_than_cut_down() {
		assert_eq!(wait(Some("90")), Duration::from_secs(90));
	}

	#[test]
	fn a_wait_past_the_longest_one_is_given_up_on_rather_than_knocked_on_early() {
		assert!(too_long(Duration::from_secs(90)));
		assert!(too_long(LONGEST_WAIT + Duration::from_secs(1)));
	}

	#[test]
	fn a_wait_the_run_can_sit_through_is_waited_out() {
		assert!(!too_long(LONGEST_WAIT));
		assert!(!too_long(Duration::from_secs(30)));
		assert!(!too_long(UNSAID_WAIT));
	}

	#[test]
	fn the_retry_after_musicbrainz_sends_is_read_before_the_reset_listenbrainz_sends() {
		let mut headers = HeaderMap::new();
		headers.append(RETRY_AFTER, HeaderValue::from_static("11"));
		headers.append(RESET_IN, HeaderValue::from_static("22"));

		assert_eq!(said(&headers), Some("11"));
	}

	#[test]
	fn the_reset_listenbrainz_sends_is_read_when_no_retry_after_stands() {
		let mut headers = HeaderMap::new();
		headers.append(RESET_IN, HeaderValue::from_static("22"));

		assert_eq!(said(&headers), Some("22"));
	}

	#[test]
	fn a_throttling_naming_no_header_at_all_is_still_waited_out() {
		assert_eq!(said(&HeaderMap::new()), None);
		assert_eq!(wait(said(&HeaderMap::new())), UNSAID_WAIT);
	}
}

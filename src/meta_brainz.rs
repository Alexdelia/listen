use std::{
	error::Error,
	fmt,
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
const MARGIN: Duration = Duration::from_secs(1);

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
	url: &str,
	attempt: impl Fn() -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	let mut taken = 0;

	loop {
		block_ready();

		let mut response = attempt().map_err(|e| ge!(format!("{failure}\n{e}")))?;
		let status = response.status();

		if throttled(status) {
			let asked = said(response.headers()).and_then(seconds);
			let wait = asked.map_or_else(|| unsaid(taken), sit_through);

			listen_agent::hold(url, wait);

			if let Some(asked) = asked.filter(|asked| too_long(*asked)) {
				return Err(gave_up_on(failure, GaveUp::AskedForLonger(asked)));
			}

			if taken < RETRY {
				taken += 1;
				thread::sleep(wait);
				continue;
			}

			return Err(gave_up_on(failure, GaveUp::StillThrottled(status)));
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

const fn sit_through(asked: Duration) -> Duration {
	asked.saturating_add(MARGIN)
}

fn unsaid(taken: u8) -> Duration {
	UNSAID_WAIT.saturating_mul(2u32.saturating_pow(u32::from(taken)))
}

fn too_long(wait: Duration) -> bool {
	wait > LONGEST_WAIT
}

#[derive(Debug)]
enum GaveUp {
	AskedForLonger(Duration),
	StillThrottled(StatusCode),
}

impl GaveUp {
	fn hint(&self) -> String {
		match self {
			Self::AskedForLonger(wait) => format!(
				"the service is asking to be left alone, run it again in {seconds}s",
				seconds = wait.as_secs()
			),
			Self::StillThrottled(_) => {
				String::from("the service is still refusing, run it again later")
			}
		}
	}
}

impl fmt::Display for GaveUp {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::AskedForLonger(wait) => write!(
				f,
				"throttled for {B}{seconds}s{D}, longer than this run waits out",
				seconds = wait.as_secs()
			),
			Self::StillThrottled(status) => write!(
				f,
				"throttled ({B}{status}{D}) through every one of the {B}{RETRY}{D} retries"
			),
		}
	}
}

impl Error for GaveUp {}

fn gave_up_on(failure: &str, reason: GaveUp) -> Box<dyn Error> {
	let hint = reason.hint();

	ge!(failure, h: hint, s: reason).into()
}

pub(crate) fn gave_up(mut e: &(dyn Error + 'static)) -> bool {
	while !e.is::<GaveUp>() {
		let Some(source) = e.source() else {
			return false;
		};

		e = source;
	}

	true
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
		assert_eq!(seconds("30"), Some(Duration::from_secs(30)));
	}

	#[test]
	fn a_named_wait_is_read_through_the_spaces_around_it() {
		assert_eq!(seconds(" 7 "), Some(Duration::from_secs(7)));
	}

	#[test]
	fn a_named_wait_longer_than_the_longest_one_is_kept_whole_rather_than_cut_down() {
		assert_eq!(seconds("90"), Some(Duration::from_secs(90)));
	}

	#[test]
	fn a_retry_after_holding_a_date_rather_than_seconds_names_no_wait_at_all() {
		assert_eq!(seconds("Wed, 21 Oct 2015 07:28:00 GMT"), None);
	}

	#[test]
	fn a_wait_a_service_names_is_sat_through_a_moment_past_what_it_asked_for() {
		assert_eq!(
			sit_through(Duration::from_secs(30)),
			Duration::from_secs(31)
		);
	}

	#[test]
	fn the_longest_wait_a_run_accepts_is_judged_on_what_was_asked_not_on_the_margin() {
		assert!(!too_long(LONGEST_WAIT));
		assert_eq!(sit_through(LONGEST_WAIT), LONGEST_WAIT + MARGIN);
	}

	#[test]
	fn the_first_wait_a_service_names_none_for_is_the_unsaid_one() {
		assert_eq!(unsaid(0), UNSAID_WAIT);
	}

	#[test]
	fn each_unsaid_wait_doubles_the_one_taken_before_it() {
		assert_eq!(unsaid(1), UNSAID_WAIT * 2);
		assert_eq!(unsaid(2), UNSAID_WAIT * 4);
		assert_eq!(unsaid(3), UNSAID_WAIT * 8);
	}

	#[test]
	fn every_unsaid_wait_a_run_can_reach_stays_within_what_it_sits_through() {
		for taken in 0..=RETRY {
			assert!(!too_long(unsaid(taken)), "unsaid({taken}) is given up on");
		}
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
	fn a_throttling_naming_no_header_at_all_names_no_wait() {
		assert_eq!(said(&HeaderMap::new()), None);
	}

	#[test]
	fn the_error_a_wait_past_the_longest_one_builds_says_the_run_gave_up() {
		let e = gave_up_on(
			"failed to submit rating",
			GaveUp::AskedForLonger(Duration::from_secs(90)),
		);

		assert!(gave_up(&*e));
	}

	#[test]
	fn the_error_a_throttling_outliving_every_retry_builds_says_the_run_gave_up() {
		let e = gave_up_on(
			"failed to submit rating",
			GaveUp::StillThrottled(StatusCode::SERVICE_UNAVAILABLE),
		);

		assert!(gave_up(&*e));
	}

	#[test]
	fn a_failure_that_is_no_throttling_does_not_say_the_run_gave_up() {
		let e: Box<dyn Error> = ge!("failed to submit rating").into();

		assert!(!gave_up(&*e));
	}

	#[test]
	fn a_giving_up_is_still_read_under_a_failure_that_wraps_it() {
		let e: Box<dyn Error> = ge!(
			"failed to submit rating",
			s: ge!("throttled", s: GaveUp::StillThrottled(StatusCode::TOO_MANY_REQUESTS))
		)
		.into();

		assert!(gave_up(&*e));
	}
}

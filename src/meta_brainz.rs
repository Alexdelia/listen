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
use chrono::{NaiveDateTime, Utc};
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

const FIXDATE: &str = "%a, %d %b %Y %H:%M:%S GMT";
const RFC_850: &str = "%A, %d-%b-%y %H:%M:%S GMT";
const ASCTIME: &str = "%a %b %e %H:%M:%S %Y";

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

#[derive(Clone, Copy)]
enum Resend {
	Safe,
	Never,
}

pub(crate) fn send(
	url: &str,
	attempt: impl Fn() -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	sent(url, attempt, failure, Resend::Safe)
}

pub(crate) fn send_once(
	url: &str,
	attempt: impl Fn() -> Result<Response<Body>, ureq::Error>,
	failure: &str,
) -> hmerr::Result<Sent> {
	sent(url, attempt, failure, Resend::Never)
}

fn sent(
	url: &str,
	attempt: impl Fn() -> Result<Response<Body>, ureq::Error>,
	failure: &str,
	resend: Resend,
) -> hmerr::Result<Sent> {
	let mut taken = 0;

	loop {
		block_ready();

		let mut response = attempt().map_err(|e| ge!(format!("{failure}\n{e}")))?;
		let status = response.status();

		if throttled(status) {
			let named = said(response.headers()).and_then(asked);
			let wait = named.map_or_else(|| unsaid(taken), sit_through);

			listen_agent::hold(url, held(wait));

			if let Some(named) = named.filter(|named| too_long(*named)) {
				return Err(gave_up_on(failure, GaveUp::AskedForLonger(named)));
			}

			if matches!(resend, Resend::Never) {
				return Err(gave_up_on(failure, GaveUp::NotSentAgain(status)));
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
	NotSentAgain(StatusCode),
	StillThrottled(StatusCode),
}

impl GaveUp {
	fn hint(&self) -> String {
		match self {
			Self::AskedForLonger(wait) => format!(
				"the service is asking to be left alone, run it again in {seconds}s",
				seconds = wait.as_secs()
			),
			Self::NotSentAgain(_) => String::from(
				"the service took the request or refused it, running it again is the only way to tell",
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
			Self::NotSentAgain(status) => write!(
				f,
				"throttled ({B}{status}{D}), and this request is not one to send twice"
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

fn held(wait: Duration) -> Duration {
	wait.min(sit_through(LONGEST_WAIT))
}

fn asked(said: &str) -> Option<Duration> {
	let said = said.trim();

	seconds(said).or_else(|| date(said))
}

fn seconds(said: &str) -> Option<Duration> {
	Some(Duration::from_secs(said.parse().ok()?))
}

fn date(said: &str) -> Option<Duration> {
	let at = [FIXDATE, RFC_850, ASCTIME]
		.into_iter()
		.find_map(|format| NaiveDateTime::parse_from_str(said, format).ok())?;

	Some(
		at.and_utc()
			.signed_duration_since(Utc::now())
			.to_std()
			.unwrap_or(Duration::ZERO),
	)
}

#[cfg(test)]
mod tests {
	use std::cell::Cell;

	use ureq::http::HeaderValue;

	use super::*;

	fn throttling(wait: &str) -> Response<Body> {
		Response::builder()
			.status(StatusCode::TOO_MANY_REQUESTS)
			.header(RETRY_AFTER, wait)
			.body(Body::builder().data(""))
			.expect("a throttling response is well formed")
	}

	fn answered() -> Response<Body> {
		Response::builder()
			.status(StatusCode::OK)
			.body(Body::builder().data("answered"))
			.expect("an answered response is well formed")
	}

	#[test]
	fn a_request_that_must_not_be_sent_twice_is_left_at_one_attempt_when_it_is_throttled() {
		let taken = Cell::new(0);

		let e = sent(
			"https://not.sent.again.test/oauth2/token",
			|| {
				taken.set(taken.get() + 1);

				Ok(throttling("1"))
			},
			"failed to reach musicbrainz oauth",
			Resend::Never,
		)
		.err()
		.expect("a throttled request that must not be sent twice gives up");

		assert_eq!(taken.get(), 1);
		assert!(gave_up(&*e));
	}

	#[test]
	fn a_request_that_can_be_sent_again_is_sent_again_once_the_wait_is_out() {
		let taken = Cell::new(0);

		let answer = sent(
			"https://sent.again.test/ws/2/rating",
			|| {
				taken.set(taken.get() + 1);

				Ok(if taken.get() == 1 {
					throttling("0")
				} else {
					answered()
				})
			},
			"failed to submit rating",
			Resend::Safe,
		)
		.expect("the attempt made once the wait is out is answered");

		assert_eq!(taken.get(), 2);
		assert_eq!(answer.status, StatusCode::OK);
		assert_eq!(answer.body, "answered");
	}

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
		assert_eq!(asked(" 7 "), Some(Duration::from_secs(7)));
	}

	#[test]
	fn a_named_wait_longer_than_the_longest_one_is_kept_whole_rather_than_cut_down() {
		assert_eq!(seconds("90"), Some(Duration::from_secs(90)));
	}

	#[test]
	fn a_retry_after_holding_a_date_is_no_number_of_seconds() {
		assert_eq!(seconds("Wed, 21 Oct 2015 07:28:00 GMT"), None);
	}

	#[test]
	fn a_retry_after_holding_a_date_is_read_as_the_wait_until_that_date() {
		let at = Utc::now() + chrono::TimeDelta::seconds(90);
		let said = at.format(FIXDATE).to_string();

		let read = asked(&said).expect("a date names the wait until it");

		assert!(read > Duration::from_secs(85) && read <= Duration::from_secs(90));
	}

	#[test]
	fn every_date_form_http_allows_is_read_as_a_wait() {
		assert_eq!(date("Sun, 06 Nov 1994 08:49:37 GMT"), Some(Duration::ZERO));
		assert_eq!(date("Sunday, 06-Nov-94 08:49:37 GMT"), Some(Duration::ZERO));
		assert_eq!(date("Sun Nov  6 08:49:37 1994"), Some(Duration::ZERO));
	}

	#[test]
	fn a_date_already_gone_by_names_no_wait_to_sit_through() {
		assert_eq!(date("Wed, 21 Oct 2015 07:28:00 GMT"), Some(Duration::ZERO));
	}

	#[test]
	fn a_date_far_past_the_longest_wait_is_given_up_on_rather_than_sat_through() {
		let at = Utc::now() + chrono::TimeDelta::hours(1);

		let read = asked(&at.format(FIXDATE).to_string()).expect("a date names a wait");

		assert!(too_long(read));
	}

	#[test]
	fn a_retry_after_that_is_neither_seconds_nor_a_date_names_no_wait_at_all() {
		assert_eq!(asked("soon"), None);
		assert_eq!(asked(""), None);
	}

	#[test]
	fn no_wait_holds_a_host_for_longer_than_this_run_would_sit_through_itself() {
		assert_eq!(held(Duration::from_secs(30)), Duration::from_secs(30));
		assert_eq!(held(sit_through(LONGEST_WAIT)), sit_through(LONGEST_WAIT));
		assert_eq!(held(Duration::from_secs(3600)), sit_through(LONGEST_WAIT));
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

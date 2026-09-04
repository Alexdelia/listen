use std::{
	collections::HashMap,
	sync::{LazyLock, Mutex, MutexGuard, PoisonError},
	thread,
	time::{Duration, Instant},
};

use ureq::{
	Body, Error, SendBody,
	http::{Request, Response, Uri},
	middleware::MiddlewareNext,
};

type Held = HashMap<Box<str>, Instant>;

static HELD: LazyLock<Mutex<Held>> = LazyLock::new(|| Mutex::new(Held::new()));

fn held() -> MutexGuard<'static, Held> {
	HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn hold(url: &str, wait: Duration) {
	let Some(host) = host(url) else {
		return;
	};

	let Some(until) = Instant::now().checked_add(wait) else {
		return;
	};

	let mut held = held();

	if held
		.get(host.as_str())
		.is_none_or(|standing| *standing < until)
	{
		held.insert(host.into_boxed_str(), until);
	}
}

pub(crate) fn gate(
	request: Request<SendBody>,
	next: MiddlewareNext,
) -> Result<Response<Body>, Error> {
	if let Some(host) = request.uri().host() {
		wait_out(host);
	}

	next.handle(request)
}

fn wait_out(host: &str) {
	while let Some(left) = left(host) {
		thread::sleep(left);
	}
}

fn left(host: &str) -> Option<Duration> {
	held().get(host)?.checked_duration_since(Instant::now())
}

fn host(url: &str) -> Option<String> {
	let uri = url.parse::<Uri>().ok()?;

	Some(uri.host()?.to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_host_nothing_was_asked_of_has_nothing_left_to_wait_out() {
		assert_eq!(left("nothing.asked.test"), None);
	}

	#[test]
	fn the_wait_a_service_asked_for_is_left_to_wait_out_under_its_host() {
		hold(
			"https://asked.test/ws/2/rating?client=x",
			Duration::from_secs(30),
		);

		let left = left("asked.test").expect("the wait just recorded stands");

		assert!(left > Duration::from_secs(29) && left <= Duration::from_secs(30));
	}

	#[test]
	fn a_wait_asked_of_one_host_leaves_every_other_host_alone() {
		hold("https://alone.test/ws/2/rating", Duration::from_secs(30));

		assert_eq!(left("beside.alone.test"), None);
	}

	#[test]
	fn a_shorter_wait_does_not_cut_the_longer_one_already_standing_short() {
		hold("https://longest.test", Duration::from_secs(60));
		hold("https://longest.test", Duration::from_secs(1));

		let left = left("longest.test").expect("the longer wait stands");

		assert!(left > Duration::from_secs(30));
	}

	#[test]
	fn a_longer_wait_takes_over_the_shorter_one_already_standing() {
		hold("https://longer.test", Duration::from_secs(1));
		hold("https://longer.test", Duration::from_secs(60));

		let left = left("longer.test").expect("the longer wait stands");

		assert!(left > Duration::from_secs(30));
	}

	#[test]
	fn a_wait_that_has_run_out_leaves_nothing_to_wait_out() {
		hold("https://ran.out.test", Duration::ZERO);

		assert_eq!(left("ran.out.test"), None);
	}

	#[test]
	fn a_url_no_host_can_be_read_out_of_holds_nothing() {
		hold("not a url at all", Duration::from_secs(30));

		assert_eq!(left("not a url at all"), None);
		assert_eq!(left(""), None);
	}

	#[test]
	fn every_request_the_agent_makes_waits_out_the_wait_standing_on_its_host() {
		let wait = Duration::from_millis(500);

		hold("http://127.0.0.1:1", wait);

		let refused = Instant::now();
		let _ = crate::shared().get("http://127.0.0.1:1/").call();

		assert!(refused.elapsed() >= wait);
	}

	#[test]
	fn the_host_a_wait_is_recorded_under_is_the_one_the_gate_reads_off_a_request() {
		assert_eq!(
			host("https://musicbrainz.org/ws/2/rating?client=x").as_deref(),
			Some("musicbrainz.org")
		);
		assert_eq!(
			host("https://api.listenbrainz.org/1/submit-listens").as_deref(),
			Some("api.listenbrainz.org")
		);
	}
}

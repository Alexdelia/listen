use std::{
	sync::Mutex,
	time::{Duration, Instant},
};

use super::token::Token;

const MARGIN: Duration = Duration::from_secs(60);

struct Held {
	access: String,
	until: Instant,
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);

pub(super) fn access() -> Option<String> {
	let held = HELD.lock().ok()?;

	held.as_ref()
		.filter(|held| fresh(held, Instant::now()))
		.map(|held| held.access.clone())
}

pub(super) fn keep(token: &Token) {
	let Some(lasts) = token.lasts else {
		return;
	};

	if let Ok(mut held) = HELD.lock() {
		*held = Some(Held {
			access: token.access.clone(),
			until: Instant::now() + lasts,
		});
	}
}

fn fresh(held: &Held, now: Instant) -> bool {
	now + MARGIN < held.until
}

#[cfg(test)]
mod tests {
	use super::*;

	fn held_for(lasts: Duration, now: Instant) -> Held {
		Held {
			access: String::from("access"),
			until: now + lasts,
		}
	}

	#[test]
	fn a_token_with_time_left_is_reused() {
		let now = Instant::now();

		assert!(fresh(&held_for(Duration::from_secs(3600), now), now));
	}

	#[test]
	fn a_token_about_to_expire_is_not_reused() {
		let now = Instant::now();

		assert!(!fresh(&held_for(MARGIN, now), now));
	}
}

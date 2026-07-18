mod consider;
mod declared;
mod declined;
mod fetch;

use std::path::Path;

use crate::cache;

pub async fn run(path: &Path, username: Option<&str>, unlistened: bool) -> hmerr::Result<()> {
	let username = cache::username::resolve(username)?;

	let mut skip = declared::sources(path)?;
	skip.extend(declined::load()?);

	let mut offset = 0;
	loop {
		let page = fetch::page(&username, offset)?;
		if page.recommendation.is_empty() {
			break;
		}

		for recommendation in &page.recommendation {
			if consider::consider(path, recommendation, unlistened, &mut skip)
				.await?
				.is_break()
			{
				return Ok(());
			}
		}

		offset += page.fetched;
		if page.fetched == 0 || offset >= page.total {
			break;
		}
	}

	Ok(())
}

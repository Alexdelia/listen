mod attraction;
mod collaborative_filtering;
mod consider;
mod declared;
mod declined;
mod feed;
mod forecast;
pub(crate) mod island;
mod known_artist;
mod labelled;
mod listen_count;
mod local;
mod log;
mod mode;
mod queue;
mod recommendation;
mod selection;
mod skip;
mod stream;
mod target;
mod trace;
mod turn;
mod weekly;

use std::path::Path;

use crate::args::{IslandArg, RecommendSort, RecommendSource};

use feed::{Fed, Feed};
use local::Local;
use mode::Mode;
use skip::Skip;
use stream::Stream;
use target::Target;

#[derive(Clone, Copy)]
pub(crate) struct Request<'a> {
	pub target: Option<&'a str>,
	pub unlistened: bool,
	pub source: RecommendSource,
	pub sort: RecommendSort,
	pub arg: &'a IslandArg,
	pub trace: Option<usize>,
}

struct Built {
	fed: Vec<Fed>,
	local: Option<Local>,
}

pub(crate) async fn run(path: &Path, request: Request<'_>) -> hmerr::Result<()> {
	selection::ensure_arg(request.source, request.arg)?;
	let mode = Mode::of(request.trace);

	let Some(Built { fed, local }) = built(path, request, &mode).await? else {
		return Ok(());
	};

	let (feed, report): (Vec<Box<dyn Feed>>, Vec<feed::Report>) =
		fed.into_iter().map(|fed| (fed.feed, fed.report)).unzip();
	let mut skip = Skip::load(path)?;
	let mut stream = Stream::new(feed, request.unlistened);

	let Mode::Trace { limit } = mode else {
		for report in &report {
			report.print();
		}

		return interact(path, &mut stream, &mut skip).await;
	};

	let count = skip.count();
	let mut recommendation = trace::collect(&mut stream, &mut skip, limit)?;
	if let Some(local) = &local {
		trace::attach_artist(&local.index.db, &mut recommendation)?;
	}

	trace::write(&trace::Document {
		arg: trace::Arg {
			target: request.target,
			source: request.source,
			sort: request.sort,
			unlistened: request.unlistened,
			limit,
			island: request.arg,
		},
		index: local.as_ref().map(|local| &local.index.meta),
		skip: count,
		feed: report,
		recommendation,
		skipped: stream.skipped(),
	})
}

async fn interact(path: &Path, stream: &mut Stream, skip: &mut Skip) -> hmerr::Result<()> {
	while let Some(shown) = stream.next(skip)? {
		if consider::consider(path, shown.index, &shown.recommendation)
			.await?
			.is_break()
		{
			return Ok(());
		}
	}

	Ok(())
}

async fn built(path: &Path, request: Request<'_>, mode: &Mode) -> hmerr::Result<Option<Built>> {
	let Request {
		target,
		source,
		sort,
		arg,
		..
	} = request;

	if selection::island_only(source) {
		selection::ensure_local_target(source, sort, target)?;
		let local = mode.open(path)?;
		let fed = vec![island::feed(&local, arg, mode.log(island::log_path)?)?];

		return Ok(Some(Built {
			fed,
			local: Some(local),
		}));
	}

	if selection::forecast_only(source) {
		selection::ensure_local_target(source, sort, target)?;
		let local = mode.open(path)?;

		if arg.backtest {
			forecast::backtest(&local)?;
			return Ok(None);
		}

		let fed = vec![forecast::feed(&local, arg, mode.log(forecast::log_path)?)?];

		return Ok(Some(Built {
			fed,
			local: Some(local),
		}));
	}

	let target = mode.target(target)?;
	selection::ensure(source, sort, &target)?;

	let mut fed = remote(&target, source, sort).await?;
	let mut local = None;

	if matches!(target, Target::Username(_))
		&& let Some((opened, local_fed)) = local_feed(path, source, arg, mode)?
	{
		fed.extend(local_fed);
		local = Some(opened);
	}

	Ok(Some(Built { fed, local }))
}

fn local_feed(
	path: &Path,
	source: RecommendSource,
	arg: &IslandArg,
	mode: &Mode,
) -> hmerr::Result<Option<(Local, Vec<Fed>)>> {
	let island = selection::island(source);
	let forecast = selection::forecast(source);

	if !island && !forecast {
		return Ok(None);
	}

	if !local::ready() {
		island::absent();
		return Ok(None);
	}

	let local = mode.open(path)?;
	let mut fed = Vec::new();

	if island {
		fed.push(island::feed(&local, arg, mode.log(island::log_path)?)?);
	}

	if forecast {
		fed.push(forecast::feed(&local, arg, mode.log(forecast::log_path)?)?);
	}

	Ok(Some((local, fed)))
}

async fn remote(
	target: &Target,
	source: RecommendSource,
	sort: RecommendSort,
) -> hmerr::Result<Vec<Fed>> {
	let mut fed = Vec::new();

	if let Target::Username(username) = target {
		if let Some(weekly) = weekly::feed(username, source)? {
			fed.push(weekly);
		}

		if selection::collaborative_filtering(source) {
			fed.push(Fed {
				feed: Box::new(collaborative_filtering::feed(username.clone())),
				report: feed::Report::CollaborativeFiltering {
					username: username.clone(),
				},
			});
		}
	}

	if let Target::Artist(mbid) = target {
		fed.push(listen_count::feed(*mbid, sort).await?);
	}

	Ok(fed)
}

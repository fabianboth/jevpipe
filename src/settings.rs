use std::num::NonZeroUsize;
use std::time::Duration;

use clap::builder::NonEmptyStringValueParser;
use clap::{Arg, Args, Command};
use humantime::DurationError;

use crate::cost::Cost;
use crate::tokens::Tokens;

const NONE: &str = "none";

#[derive(Args)]
pub(crate) struct Settings {
    /// Model to ask; pin a version such as jev-1.13 (OpenRouter) or jev-1.13.0 (TypeSafe)
    #[arg(long, default_value = "jev-latest", value_parser = NonEmptyStringValueParser::new())]
    pub(crate) model: String,

    /// Maximum requests in flight
    #[arg(long, value_name = "N", default_value = "100")]
    pub(crate) concurrency: NonZeroUsize,

    /// Give up on a request after this long, retries included, e.g. 10s or 1m
    #[arg(long, value_name = "DURATION", default_value = "10s", value_parser = duration)]
    pub(crate) request_timeout: Duration,

    /// Stop sending requests once this run's reported cost reaches this many US dollars
    #[arg(long, value_name = "DOLLARS|none", default_value = NONE, value_parser = limit(dollars))]
    pub(crate) max_cost: Limit<Cost>,

    /// Stop sending requests once this run's reported tokens reach COUNT, e.g. 250k or 5M
    #[arg(long, value_name = "COUNT|none", default_value = NONE, value_parser = limit(count))]
    pub(crate) max_tokens: Limit<Tokens>,

    /// Stop the run once it has taken this long, e.g. 90s, 10m or 1h30m
    #[arg(long, value_name = "DURATION|none", default_value = NONE, value_parser = limit(duration))]
    pub(crate) max_time: Limit<Duration>,
}

#[derive(Clone, Copy)]
pub(crate) enum Limit<T> {
    Unlimited,
    At(T),
}

impl<T> Limit<T> {
    pub(crate) fn map<U>(self, convert: impl FnOnce(T) -> U) -> Limit<U> {
        match self {
            Self::Unlimited => Limit::Unlimited,
            Self::At(value) => Limit::At(convert(value)),
        }
    }
}

fn command() -> Command {
    Settings::augment_args(Command::new("settings")).no_binary_name(true)
}

pub(crate) fn keys() -> Vec<String> {
    command()
        .get_arguments()
        .filter_map(Arg::get_long)
        .map(str::to_owned)
        .collect()
}

pub(crate) fn default(key: &str) -> Option<String> {
    command()
        .get_arguments()
        .find(|arg| arg.get_long() == Some(key))?
        .get_default_values()
        .first()
        .map(|value| value.to_string_lossy().into_owned())
}

pub(crate) fn with_defaults<'a>(
    command: Command,
    values: impl Iterator<Item = (&'a str, &'a str)>,
) -> Command {
    values.fold(command, |command, (key, value)| {
        let id = command
            .get_arguments()
            .find(|arg| arg.get_long() == Some(key))
            .map(|arg| arg.get_id().clone());
        match id {
            Some(id) => command.mut_arg(id, |arg| arg.default_value(value.to_owned())),
            None => command,
        }
    })
}

pub(crate) fn check(key: &str, value: &str) -> Result<(), String> {
    command()
        .try_get_matches_from([format!("--{key}={value}")])
        .map(drop)
        .map_err(|error| {
            std::error::Error::source(&error)
                .map_or_else(|| error.kind().to_string(), ToString::to_string)
        })
}

fn duration(value: &str) -> Result<Duration, String> {
    match humantime::parse_duration(value) {
        Ok(duration) if duration.is_zero() => Err("must be longer than zero".to_owned()),
        Ok(duration) => Ok(duration),
        Err(DurationError::UnknownUnit { unit, .. }) if unit.is_empty() => {
            Err("needs a unit, for example 10s or 5m".to_owned())
        }
        Err(
            error @ (DurationError::InvalidCharacter(_)
            | DurationError::NumberExpected(_)
            | DurationError::UnknownUnit { .. }
            | DurationError::NumberOverflow
            | DurationError::Empty),
        ) => Err(error.to_string()),
    }
}

fn dollars(value: &str) -> Result<Cost, String> {
    match value.parse::<Cost>() {
        Ok(cost) if cost.is_zero() => Err("must be more than zero".to_owned()),
        Ok(cost) => Ok(cost),
        Err(error) => Err(error.to_string()),
    }
}

fn count(value: &str) -> Result<Tokens, String> {
    match value.parse::<Tokens>() {
        Ok(tokens) if tokens.is_zero() => Err("must be positive".to_owned()),
        Ok(tokens) => Ok(tokens),
        Err(error) => Err(error.to_string()),
    }
}

fn limit<T>(
    parse: fn(&str) -> Result<T, String>,
) -> impl Fn(&str) -> Result<Limit<T>, String> + Clone + Send + Sync + 'static
where
    T: 'static,
{
    move |value| match value {
        NONE => Ok(Limit::Unlimited),
        value => parse(value).map(Limit::At),
    }
}

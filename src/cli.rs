use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Parser, Subcommand};

use crate::decision::PROBABILITY;

const FILTER_EXAMPLES: &str = r#"Needs OPENROUTER_API_KEY in the environment.

Examples:
  git ls-files | jevpipe filter "Does this file parse command line arguments?" --read-files
  jevpipe filter "Is this line an error?" app.log --json --all | jq ."#;

/// A Unix pipe for typed decisions: stream records in, get calibrated decisions out.
#[derive(Parser)]
#[command(version, bin_name = "jevpipe")]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Keep the records for which the answer to a yes/no question is yes
    ///
    /// Like grep, but the match is a question: prints the lines answered yes, unchanged and in
    /// input order. With --read-files, each line is a file path and the file's content is judged.
    ///
    /// Exit status: 0 when something was kept, 1 when nothing was, 2 when a record failed or the
    /// run stopped on an error. A one-line summary goes to standard error.
    #[command(after_help = FILTER_EXAMPLES)]
    Filter(FilterArgs),
}

#[derive(Args)]
pub(crate) struct FilterArgs {
    /// The yes/no question asked about every record
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub(crate) question: String,

    /// Files to read records from, in order; none or - reads standard input
    pub(crate) files: Vec<PathBuf>,

    /// Treat each record as a file path: judge the file's path and content, print the path
    #[arg(long)]
    pub(crate) read_files: bool,

    /// Keep a record when the probability of yes is at least this (0 to 1)
    #[arg(long, value_name = "P", default_value_t = 0.5, value_parser = probability)]
    pub(crate) threshold: f64,

    /// Print kept records as JSON objects with position, record and probability
    #[arg(long)]
    pub(crate) json: bool,

    /// With --json: print every record with its outcome (kept, dropped, skipped, failed)
    #[arg(long, requires = "json")]
    pub(crate) all: bool,

    /// Maximum requests in flight
    #[arg(long, value_name = "N", default_value = "100")]
    pub(crate) concurrency: NonZeroUsize,

    /// Model to ask; pin a version such as typesafe/jev-1.13 for reproducible runs
    #[arg(long, default_value = "~typesafe/jev-latest")]
    pub(crate) model: String,

    /// Abandon a request after this many seconds and retry it
    #[arg(long, value_name = "SECS", default_value = "10")]
    pub(crate) request_timeout: NonZeroU64,
}

fn probability(value: &str) -> Result<f64, String> {
    let number: f64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number"))?;
    if PROBABILITY.contains(&number) {
        Ok(number)
    } else {
        Err(format!("`{value}` is not between 0 and 1"))
    }
}

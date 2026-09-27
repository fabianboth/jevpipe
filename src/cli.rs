use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::answers::PROBABILITY;
use crate::filter;
use crate::questions::Questions;

const FILTER_EXAMPLES: &str = r#"Needs OPENROUTER_API_KEY in the environment.

Examples:
  git ls-files | jevpipe filter "Does this file parse command line arguments?" --read-files
  jevpipe filter "Is this line an error worth a closer look?" app.log | head -20"#;

const MAP_EXAMPLES: &str = r#"Needs OPENROUTER_API_KEY in the environment.

Questions file: named questions in TypeSafe's System One format, sent unchanged. A noul is a yes/no
question answered with a probability; a choice picks one of its criteria (option: description, 1 to
255 options); a score rates on its criteria (levels from low to high, 2 to 10):
  {
    "relevant": {"type": "noul", "instructions": "Is this failure worth a closer look?"},
    "kind": {"type": "choice", "instructions": "What kind of failure is this?",
             "criteria": {"flaky": "infra or timing", "real": "deterministic bug"}},
    "severity": {"type": "score", "instructions": "How severe is this failure?",
                 "criteria": ["cosmetic", "annoying", "blocking"]}
  }

Output, one line per record:
  {"record": "<line>", "answers": {
     "relevant": {"type": "noul", "noul": 0.82},
     "kind": {"type": "choice", "choice": "flaky", "probabilities": {...}, "confidence": 0.9},
     "severity": {"type": "score", "score": 1.04, "legend": {...}, "probabilities": {...}, ...}}}
  {"record": "<line>", "outcome": "skipped" or "failed", "reason": "<why>"}
"truncated": true is added when --read-files had to cut the file.

Examples:
  jevpipe map triage.json failures.log | jq -r 'select(.answers.kind.choice == "flaky") | .record'
  git ls-files | jevpipe map triage.json --read-files | jq -c '{record, severity: .answers.severity.score}'"#;

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

    /// Ask several typed questions about every record and print the answers as JSON lines
    ///
    /// Each record is sent once with all questions of the questions file. Prints one JSON line per
    /// record, in input order, as soon as it is answered: pipe it through jq to select and project.
    /// With --read-files, each line is a file path and the file's path and content are judged.
    ///
    /// Exit status: 0 when no record failed, 2 when one did or the run stopped on an error. A
    /// one-line summary goes to standard error.
    #[command(after_help = MAP_EXAMPLES)]
    Map(MapArgs),
}

#[derive(Args)]
pub(crate) struct FilterArgs {
    /// The yes/no question asked about every record
    #[arg(value_name = "QUESTION", value_parser = filter::question)]
    pub(crate) question: Questions,

    /// Files to read records from, in order; none or - reads standard input
    pub(crate) files: Vec<PathBuf>,

    /// Keep a record when the probability of yes is at least this (0 to 1)
    #[arg(long, value_name = "P", default_value_t = 0.5, value_parser = probability)]
    pub(crate) threshold: f64,

    #[command(flatten)]
    pub(crate) run: RunArgs,
}

#[derive(Args)]
pub(crate) struct MapArgs {
    /// JSON file of named questions (format below)
    #[arg(value_name = "QUESTIONS_FILE", value_parser = Questions::load)]
    pub(crate) questions: Questions,

    /// Files to read records from, in order; none or - reads standard input
    pub(crate) files: Vec<PathBuf>,

    #[command(flatten)]
    pub(crate) run: RunArgs,
}

#[derive(Args)]
pub(crate) struct RunArgs {
    /// Treat each record as a file path: judge the file's path and content
    #[arg(long)]
    pub(crate) read_files: bool,

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

use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand};

use crate::answers::PROBABILITY;
use crate::filter;
use crate::questions::Questions;

const FILTER_EXAMPLES: &str = r#"Examples:
  git ls-files | jevpipe filter "Does this file parse command line arguments?" --read-files
  jevpipe filter "Is this line an error worth a closer look?" app.log | head -20"#;

macro_rules! map_examples {
    () => {
        r#"Examples:
  jevpipe map -f triage.json failures.log | jq -r 'select(.answers.kind.choice == "flaky") | .record'
  jevpipe map -q '{"error": {"type": "noul", "instructions": "Is this line an error?"}}' app.log"#
    };
}

const MAP_EXAMPLES: &str = map_examples!();

const MAP_HELP: &str = concat!(
    r#"Questions (-q or -f): a JSON object of named questions in TypeSafe's System One format, e.g.
  {"relevant": {"type": "noul", "instructions": "Worth a closer look?"},
   "kind": {"type": "choice", "instructions": "What kind?", "criteria": {"flaky": "timing", "real": "bug"}},
   "severity": {"type": "score", "instructions": "How severe?", "criteria": ["low", "medium", "high"]}}
A noul is yes/no, a choice picks one of 1-255 options, a score rates on 2-10 levels (low to high).

Output, one JSON line per record: {"record": "<line>", "answers": {"<name>": {...}, ...}}
or {"record": "<line>", "outcome": "skipped" | "failed", "reason": "<why>"}

"#,
    map_examples!()
);

/// A Unix pipe for typed decisions: stream records in, get calibrated decisions out.
#[derive(Parser)]
#[command(version, bin_name = "jevpipe")]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
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
    /// Each record is sent once with all the questions. Prints one JSON line per
    /// record, in input order, as soon as it is answered: pipe it through jq to select and project.
    /// With --read-files, each line is a file path and the file's path and content are judged.
    ///
    /// Exit status: 0 when no record failed, 2 when one did or the run stopped on an error. A
    /// one-line summary goes to standard error.
    #[command(after_help = MAP_EXAMPLES, after_long_help = MAP_HELP)]
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
    #[command(flatten)]
    pub(crate) questions: QuestionsSource,

    /// Files to read records from, in order; none or - reads standard input
    pub(crate) files: Vec<PathBuf>,

    #[command(flatten)]
    pub(crate) run: RunArgs,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub(crate) struct QuestionsSource {
    /// The questions as JSON (format below)
    #[arg(short = 'q', long = "questions", value_name = "JSON", value_parser = Questions::parse)]
    inline: Option<Questions>,

    /// Read the questions from this JSON file (format below)
    #[arg(short = 'f', long = "questions-file", value_name = "FILE", value_parser = Questions::load)]
    file: Option<Questions>,
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

impl QuestionsSource {
    pub(crate) fn into_questions(self) -> Result<Questions, clap::Error> {
        self.inline.or(self.file).ok_or_else(|| {
            Cli::command().error(
                ErrorKind::MissingRequiredArgument,
                "give the questions with --questions or --questions-file",
            )
        })
    }
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

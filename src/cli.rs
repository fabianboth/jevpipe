use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::answers::PROBABILITY;
use crate::config::{self, Config};
use crate::filter;
use crate::questions::Questions;
use crate::settings::{self, Settings};

macro_rules! filter_examples {
    () => {
        r#"Examples:
  git ls-files | jevpipe filter "Does this file parse command line arguments?" --read-files
  jevpipe filter "Is this line an error worth a closer look?" app.log | head -20"#
    };
}

macro_rules! stopped_and_failed {
    () => {
        "  2  a record failed, or an error stopped the run
  3  a limit stopped the run early; standard error names the line to resume from
"
    };
}

const FILTER_EXAMPLES: &str = filter_examples!();

const FILTER_HELP: &str = concat!(
    "Exit status:
  0  at least one record was kept
  1  no record was kept
",
    stopped_and_failed!(),
    "
",
    filter_examples!()
);

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

Exit status:
  0  no record failed
"#,
    stopped_and_failed!(),
    "
",
    map_examples!()
);

const CONFIG_HELP: &str = r"Show or change the defaults for filter and map in the user config file

Each option of filter and map that has a default is a key of the same name, such as max-cost.
provider is the service answering: openrouter (default) or typesafe, TypeSafe's own API.
base-url is the service's address (default https://openrouter.ai/api, or
https://api.typesafe.ai with provider typesafe). An option on the command line beats the file,
and the file beats the built-in default.

A key prefixed with a provider applies only while that provider is in use and beats the same key
without prefix, e.g. jevpipe config set openrouter.max-cost 0.5 or typesafe.model jev-1.13.0.
The file keeps them in an [openrouter] or a [typesafe] section.

The file is config.toml in the user config directory: %APPDATA%\jevpipe on Windows,
$XDG_CONFIG_HOME/jevpipe or ~/.config/jevpipe elsewhere. JEVPIPE_CONFIG names another file.";

const AUTH_HELP: &str = r#"Store or remove the API key of the configured provider in the system keychain

Each provider has its own key. filter and map take it from OPENROUTER_API_KEY (provider
openrouter) or TYPESAFE_API_KEY (provider typesafe) when that is set, otherwise from the keychain:
Credential Manager on Windows, the login keychain on macOS, the Secret Service on Linux.

Examples:
  jevpipe auth set-key
  echo "$OPENROUTER_API_KEY" | jevpipe auth set-key
  jevpipe config set provider typesafe && echo "$TYPESAFE_API_KEY" | jevpipe auth set-key"#;

const SET_KEY_EXAMPLE: &str = r#"Example:
  echo "$OPENROUTER_API_KEY" | jevpipe auth set-key"#;

/// A Unix pipe for typed decisions: stream records in, get calibrated decisions out.
#[derive(Parser)]
#[command(version, bin_name = "jevpipe")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Keep the records for which the answer to a yes/no question is yes
    ///
    /// Like grep, but the match is a question: prints the lines answered yes, unchanged and in
    /// input order. With --read-files, each line is a file path and the file's path and content
    /// are judged. A one-line summary goes to standard error.
    #[command(after_help = FILTER_EXAMPLES, after_long_help = FILTER_HELP)]
    Filter(FilterArgs),

    /// Ask several typed questions about every record and print the answers as JSON lines
    ///
    /// Each record is sent once with all the questions. Prints one JSON line per
    /// record, in input order, as soon as it is answered: pipe it through jq to select and project.
    /// With --read-files, each line is a file path and the file's path and content are judged.
    /// A one-line summary goes to standard error.
    #[command(after_help = MAP_EXAMPLES, after_long_help = MAP_HELP)]
    Map(MapArgs),

    /// Show or change the defaults for filter and map in the user config file
    #[command(subcommand, long_about = CONFIG_HELP)]
    Config(ConfigCommand),

    /// Store or remove the API key in the system keychain
    #[command(subcommand, long_about = AUTH_HELP)]
    Auth(AuthCommand),
}

#[derive(Subcommand)]
pub(crate) enum AuthCommand {
    /// Read the API key from standard input and store it in the keychain, replacing a stored key
    ///
    /// On a terminal it asks for the key and hides the typing; a piped key is read without asking.
    #[command(after_help = SET_KEY_EXAMPLE)]
    SetKey,

    /// Remove the stored API key from the keychain
    RemoveKey,
}

#[derive(Subcommand)]
pub(crate) enum ConfigCommand {
    /// Print every setting with its value and where it comes from, as TOML
    List,

    /// Print the value of one setting
    Get {
        #[arg(help = config::key_names())]
        key: String,
    },

    /// Store a setting in the config file, checked like the option
    Set {
        #[arg(help = config::key_names())]
        key: String,

        /// The value, written as for the option, e.g. 20s or none
        #[arg(allow_hyphen_values = true)]
        value: String,
    },

    /// Remove a setting from the config file, so its default applies again
    Unset {
        /// The key to remove; a misspelled key can be removed too
        key: String,
    },

    /// Print the path of the config file, whether or not it exists
    Path,
}

#[derive(Args)]
pub(crate) struct FilterArgs {
    /// The yes/no question asked about every record
    #[arg(value_name = "QUESTION", value_parser = filter::question)]
    pub(crate) question: Questions,

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
    /// Files to read records from, in order; none or - reads standard input
    pub(crate) files: Vec<PathBuf>,

    /// Treat each record as a file path: judge the file's path and content
    #[arg(long)]
    pub(crate) read_files: bool,

    #[command(flatten)]
    pub(crate) settings: Settings,
}

impl Cli {
    pub(crate) fn parse_with(config: Option<&Config>) -> Self {
        let mut command = Self::command();
        if let Some(config) = config {
            let names: Vec<_> = command
                .get_subcommands()
                .map(|subcommand| subcommand.get_name().to_owned())
                .collect();
            for name in names {
                command = command.mut_subcommand(name, |subcommand| {
                    settings::with_defaults(subcommand, config.settings())
                });
            }
        }
        let matches = command.get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
    }
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

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::ExitCode;
use std::time::Duration;

use futures::StreamExt;

use crate::cli::RunArgs;
use crate::decision::{Decision, Outcome};
use crate::file::{self, Unjudged};
use crate::output::{Delivery, Output, report};
use crate::questions::Questions;
use crate::reason::Failure;
use crate::record::{self, FailedInput, Input, Record};
use crate::service::{MissingApiKey, Service, ServiceConfig, ServiceError, State};
use crate::summary::{Exit, Summary};

pub(crate) trait Command {
    const RESULTS: &'static str;
    const WITHOUT_RESULTS: Exit;

    fn questions(&self) -> &Questions;

    fn is_result(&self, outcome: &Outcome) -> bool;

    fn write(&self, decision: &Decision, out: &mut impl Write) -> io::Result<()>;
}

#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error(transparent)]
    MissingApiKey(#[from] MissingApiKey),
    #[error("cannot start the HTTP client: {0}")]
    Client(#[from] reqwest::Error),
    #[error("cannot read input: {0}")]
    Input(io::Error),
    #[error("cannot write output: {0}")]
    Output(io::Error),
    #[error("service error: {0}")]
    Rejected(String),
}

enum Decided {
    Record(Decision),
    Input(FailedInput),
}

struct Judge<'a> {
    service: Service,
    questions: &'a Questions,
    subject: Subject,
}

#[derive(Clone, Copy)]
enum Subject {
    Line,
    FileContent,
}

pub(crate) async fn run<C: Command>(command: C, files: Vec<PathBuf>, args: RunArgs) -> ExitCode {
    match decide_all(&command, files, args).await {
        Ok(summary) => {
            report(&summary);
            summary.exit(C::WITHOUT_RESULTS).into()
        }
        Err(error) => {
            report(format_args!("error: {error}"));
            Exit::Error.into()
        }
    }
}

async fn decide_all<C: Command>(
    command: &C,
    files: Vec<PathBuf>,
    args: RunArgs,
) -> Result<Summary, RunError> {
    let judge = Judge::new(command.questions(), &args)?;
    let mut output = Output::new();
    let inputs = record::read(files).map_err(RunError::Input)?;
    let mut decided = pin!(
        inputs
            .map(|input| judge.decide(input))
            .buffered(args.concurrency.get())
    );
    let mut summary = Summary::start(C::RESULTS);
    while let Some(decided) = decided.next().await {
        match decided? {
            Decided::Record(decision) => {
                summary.add(&decision.outcome, command.is_result(&decision.outcome));
                if let Outcome::Failed(reason) = &decision.outcome {
                    report(format_args!("line {}: {reason}", decision.record.line));
                }
                let delivery = output
                    .write(|out| command.write(&decision, out))
                    .map_err(RunError::Output)?;
                match delivery {
                    Delivery::Open => {}
                    Delivery::Closed => {
                        summary.close();
                        break;
                    }
                }
            }
            Decided::Input(input) => {
                report(format_args!("{}: {}", input.name, input.failure));
                summary.add(&Outcome::Failed(input.failure), false);
            }
        }
    }
    Ok(summary)
}

impl<'a> Judge<'a> {
    fn new(questions: &'a Questions, args: &RunArgs) -> Result<Self, RunError> {
        let config = ServiceConfig::from_env()?;
        let timeout = Duration::from_secs(args.request_timeout.get());
        Ok(Self {
            service: Service::new(config, args.model.clone(), timeout)?,
            questions,
            subject: if args.read_files {
                Subject::FileContent
            } else {
                Subject::Line
            },
        })
    }

    async fn decide(&self, input: Input) -> Result<Decided, RunError> {
        match input {
            Input::Record(record) => {
                let outcome = self.judge(&record).await?;
                Ok(Decided::Record(Decision { record, outcome }))
            }
            Input::NotText(record) => Ok(Decided::Record(Decision {
                record,
                outcome: Outcome::Failed(Failure::NotText),
            })),
            Input::Failed(input) => Ok(Decided::Input(input)),
        }
    }

    async fn judge(&self, record: &Record) -> Result<Outcome, RunError> {
        match self.subject {
            Subject::Line => self.ask(&State::Text(&record.text), false).await,
            Subject::FileContent => match file::read(Path::new(&record.text)).await {
                Ok(content) => {
                    let state = State::File {
                        path: &record.text,
                        content: &content.text,
                    };
                    self.ask(&state, content.truncated).await
                }
                Err(Unjudged::Skipped(reason)) => Ok(Outcome::Skipped(reason)),
                Err(Unjudged::Failed(reason)) => Ok(Outcome::Failed(reason)),
            },
        }
    }

    async fn ask(&self, state: &State<'_>, truncated: bool) -> Result<Outcome, RunError> {
        match self.service.ask(self.questions, state).await {
            Ok(reply) => Ok(Outcome::Answered { reply, truncated }),
            Err(ServiceError::Transient { .. }) => Ok(Outcome::Failed(Failure::ServiceUnavailable)),
            Err(ServiceError::TooLarge) => Ok(Outcome::Failed(Failure::TooLarge)),
            Err(ServiceError::Rejected(message)) => Err(RunError::Rejected(message)),
        }
    }
}

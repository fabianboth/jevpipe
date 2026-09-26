use std::io;
use std::path::Path;
use std::pin::pin;
use std::process::ExitCode;
use std::time::Duration;

use futures::StreamExt;

use crate::cli::FilterArgs;
use crate::decision::{Decision, Failure, Judgment, Outcome};
use crate::file;
use crate::output::{Delivery, Format, Output, report};
use crate::record::{self, Incoming, Record};
use crate::service::{MissingApiKey, Service, ServiceConfig, ServiceError, State};
use crate::summary::{Exit, Summary};

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

struct Judge {
    service: Service,
    question: String,
    threshold: f64,
    subject: Subject,
}

#[derive(Clone, Copy)]
enum Subject {
    Line,
    FileContent,
}

pub(crate) async fn run(args: FilterArgs) -> ExitCode {
    match decide_all(args).await {
        Ok(summary) => {
            report(&summary);
            summary.exit().into()
        }
        Err(error) => {
            report(format_args!("error: {error}"));
            Exit::Error.into()
        }
    }
}

async fn decide_all(args: FilterArgs) -> Result<Summary, RunError> {
    let judge = Judge::new(&args)?;
    let mut output = Output::new(Format::new(args.json, args.all));
    let records = record::read(args.files).map_err(RunError::Input)?;
    let mut decisions = pin!(
        records
            .map(|incoming| judge.decide(incoming))
            .buffered(args.concurrency.get())
    );
    let mut summary = Summary::start();
    while let Some(decision) = decisions.next().await {
        let decision = decision?;
        summary.add(&decision);
        match output.write(&decision).map_err(RunError::Output)? {
            Delivery::Open => {}
            Delivery::Closed => {
                summary.close();
                break;
            }
        }
    }
    Ok(summary)
}

impl Judge {
    fn new(args: &FilterArgs) -> Result<Self, RunError> {
        let config = ServiceConfig::from_env()?;
        let timeout = Duration::from_secs(args.request_timeout.get());
        Ok(Self {
            service: Service::new(config, args.model.clone(), timeout)?,
            question: args.question.clone(),
            threshold: args.threshold,
            subject: if args.read_files {
                Subject::FileContent
            } else {
                Subject::Line
            },
        })
    }

    async fn decide(&self, incoming: Incoming) -> Result<Decision, RunError> {
        match incoming {
            Ok(record) => {
                let outcome = self.judge(&record).await?;
                Ok(Decision { record, outcome })
            }
            Err(decided) => Ok(decided),
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
                Err(outcome) => Ok(outcome),
            },
        }
    }

    async fn ask(&self, state: &State<'_>, truncated: bool) -> Result<Outcome, RunError> {
        match self.service.ask(&self.question, state).await {
            Ok(answer) => {
                let judgment = Judgment {
                    probability: answer.probability,
                    truncated,
                    cost: answer.cost,
                    model: answer.model,
                };
                Ok(Outcome::judged(judgment, self.threshold))
            }
            Err(ServiceError::Transient { .. }) => Ok(Outcome::failed(Failure::ServiceUnavailable)),
            Err(ServiceError::TooLarge) => Ok(Outcome::failed(Failure::TooLarge)),
            Err(ServiceError::Rejected(message)) => Err(RunError::Rejected(message)),
        }
    }
}

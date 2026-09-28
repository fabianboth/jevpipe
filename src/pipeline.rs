use std::io::{self, Write};
use std::path::Path;
use std::pin::pin;
use std::process::ExitCode;

use futures::StreamExt;

use crate::auth::{self, LookupError};
use crate::cli::RunArgs;
use crate::config::Config;
use crate::decision::{Decision, Outcome};
use crate::exit::{self, Exit};
use crate::file::{self, Content, Unjudged};
use crate::limits::{Limits, NoCost, Stop, TooLong};
use crate::output::{Delivery, Output, report};
use crate::questions::Questions;
use crate::reason::Failure;
use crate::record::{self, FailedInput, Input, Record};
use crate::service::{Service, ServiceConfig, ServiceError, State};
use crate::summary::Summary;

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
    ApiKey(#[from] LookupError),
    #[error("cannot start the HTTP client: {0}")]
    Client(#[from] reqwest::Error),
    #[error("cannot read input: {0}")]
    Input(io::Error),
    #[error("cannot write output: {0}")]
    Output(io::Error),
    #[error("service error: {0}")]
    Rejected(String),
    #[error(transparent)]
    NoCost(#[from] NoCost),
    #[error(transparent)]
    TooLong(#[from] TooLong),
}

enum Decided {
    Record(Decision),
    Input(FailedInput),
    Unprocessed,
}

const HALVINGS: usize = 3;

enum Judged {
    Outcome(Outcome),
    Unprocessed,
}

impl Judged {
    fn is_too_large(&self) -> bool {
        matches!(self, Self::Outcome(Outcome::Failed(Failure::TooLarge)))
    }
}

struct Judge<'a> {
    service: Service,
    questions: &'a Questions,
    subject: Subject,
    limits: Limits,
}

#[derive(Clone, Copy)]
enum Subject {
    Line,
    FileContent,
}

struct Printer<'a, C> {
    command: &'a C,
    output: Output,
    summary: Summary,
    resume_line: usize,
    printing: bool,
}

pub(crate) async fn run<C: Command>(command: C, args: RunArgs, config: &Config) -> ExitCode {
    match decide_all(&command, args, config).await {
        Ok(summary) => {
            if let Some(stop) = summary.stop() {
                report(stop);
            }
            report(&summary);
            summary.exit(C::WITHOUT_RESULTS).into()
        }
        Err(error) => exit::fail(error),
    }
}

async fn decide_all<C: Command>(
    command: &C,
    args: RunArgs,
    config: &Config,
) -> Result<Summary, RunError> {
    let judge = Judge::new(command.questions(), &args, config)?;
    let limits = &judge.limits;
    let inputs = record::read(args.files)
        .map_err(RunError::Input)?
        .take_until(limits.until_stopped());
    let mut decided = pin!(
        inputs
            .map(|input| judge.decide(input))
            .buffered(args.settings.concurrency.get())
            .take_until(limits.until_deadline())
    );
    let mut printer = Printer::new(command);
    while let Some(decided) = decided.next().await {
        match printer.take(decided?)? {
            Delivery::Open => {}
            Delivery::Closed => break,
        }
    }
    Ok(printer.finish(limits))
}

impl<'a, C: Command> Printer<'a, C> {
    fn new(command: &'a C) -> Self {
        Self {
            command,
            output: Output::new(),
            summary: Summary::start(C::RESULTS),
            resume_line: 1,
            printing: true,
        }
    }

    fn take(&mut self, decided: Decided) -> Result<Delivery, RunError> {
        if !self.printing {
            return Ok(Delivery::Open);
        }
        match decided {
            Decided::Record(decision) => return self.print(&decision),
            Decided::Input(input) => {
                report(format_args!("{}: {}", input.name, input.failure));
                self.summary.add(&Outcome::Failed(input.failure));
            }
            Decided::Unprocessed => self.printing = false,
        }
        Ok(Delivery::Open)
    }

    fn print(&mut self, decision: &Decision) -> Result<Delivery, RunError> {
        self.resume_line = decision.record.line + 1;
        self.summary.add(&decision.outcome);
        if self.command.is_result(&decision.outcome) {
            self.summary.add_result();
        }
        if let Outcome::Failed(reason) = &decision.outcome {
            report(format_args!("line {}: {reason}", decision.record.line));
        }
        let delivery = self
            .output
            .write(|out| self.command.write(decision, out))
            .map_err(RunError::Output)?;
        match delivery {
            Delivery::Open => {}
            Delivery::Closed => self.summary.close(),
        }
        Ok(delivery)
    }

    fn finish(mut self, limits: &Limits) -> Summary {
        self.summary
            .finish(limits.reported_cost(), limits.stop_line(self.resume_line));
        self.summary
    }
}

impl<'a> Judge<'a> {
    fn new(questions: &'a Questions, args: &RunArgs, config: &Config) -> Result<Self, RunError> {
        let service = ServiceConfig::new(config.base_url(), auth::api_key()?);
        Ok(Self {
            service: Service::new(service, &args.settings)?,
            questions,
            subject: if args.read_files {
                Subject::FileContent
            } else {
                Subject::Line
            },
            limits: Limits::new(&args.settings)?,
        })
    }

    async fn decide(&self, input: Input) -> Result<Decided, RunError> {
        match input {
            Input::Record(record) => match self.judge(&record).await? {
                Judged::Outcome(outcome) => Ok(Decided::Record(Decision { record, outcome })),
                Judged::Unprocessed => Ok(Decided::Unprocessed),
            },
            Input::Invalid(record, failure) => Ok(Decided::Record(Decision {
                record,
                outcome: Outcome::Failed(failure),
            })),
            Input::Failed(input) => Ok(Decided::Input(input)),
        }
    }

    async fn judge(&self, record: &Record) -> Result<Judged, RunError> {
        match self.subject {
            Subject::Line => self.ask(&State::Text(&record.text), false).await,
            Subject::FileContent => self.judge_file(&record.text).await,
        }
    }

    async fn judge_file(&self, path: &str) -> Result<Judged, RunError> {
        match file::read(Path::new(path)).await {
            Ok(mut content) => {
                for _ in 0..HALVINGS {
                    let judged = self.ask_file(path, &content).await?;
                    if !judged.is_too_large() || !content.halve() {
                        return Ok(judged);
                    }
                }
                self.ask_file(path, &content).await
            }
            Err(Unjudged::Skipped(reason)) => Ok(Judged::Outcome(Outcome::Skipped(reason))),
            Err(Unjudged::Failed(reason)) => Ok(Judged::Outcome(Outcome::Failed(reason))),
        }
    }

    async fn ask_file(&self, path: &str, content: &Content) -> Result<Judged, RunError> {
        let state = State::File {
            path,
            content: &content.text,
        };
        self.ask(&state, content.truncated).await
    }

    async fn ask(&self, state: &State<'_>, truncated: bool) -> Result<Judged, RunError> {
        if !self.limits.may_send() {
            return Ok(Judged::Unprocessed);
        }
        let outcome = match self.service.ask(self.questions, state).await {
            Ok(reply) => {
                self.limits.add(reply.cost)?;
                Outcome::Answered { reply, truncated }
            }
            Err(ServiceError::Transient { .. }) => Outcome::Failed(Failure::ServiceUnavailable),
            Err(ServiceError::TooLarge) => Outcome::Failed(Failure::TooLarge),
            Err(ServiceError::Exhausted(exhausted)) => {
                self.limits.stop(Stop::Exhausted(exhausted));
                return Ok(Judged::Unprocessed);
            }
            Err(ServiceError::Rejected(message)) => return Err(RunError::Rejected(message)),
        };
        Ok(Judged::Outcome(outcome))
    }
}

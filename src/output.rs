use std::fmt::Display;
use std::io::{self, StdoutLock, Write};

use serde::Serialize;

use crate::decision::{Decision, Outcome};

pub(crate) struct Output {
    stdout: StdoutLock<'static>,
    format: Format,
}

#[derive(Clone, Copy)]
pub(crate) enum Format {
    Raw,
    Json,
    All,
}

pub(crate) enum Delivery {
    Open,
    Closed,
}

#[derive(Serialize)]
struct JsonLine<'a> {
    position: usize,
    record: &'a str,
    #[serde(flatten)]
    outcome: &'a Outcome,
}

impl Format {
    pub(crate) fn new(json: bool, all: bool) -> Self {
        match (json, all) {
            (true, true) => Self::All,
            (true, false) => Self::Json,
            (false, _) => Self::Raw,
        }
    }
}

impl Output {
    pub(crate) fn new(format: Format) -> Self {
        Self {
            stdout: io::stdout().lock(),
            format,
        }
    }

    pub(crate) fn write(&mut self, decision: &Decision) -> io::Result<Delivery> {
        if let Outcome::Failed { reason } = &decision.outcome {
            report(format_args!(
                "record {} ({}): {reason}",
                decision.record.position, decision.record.text
            ));
        }
        match self.emit(decision) {
            Ok(()) => Ok(Delivery::Open),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(Delivery::Closed),
            Err(error) => Err(error),
        }
    }

    fn emit(&mut self, decision: &Decision) -> io::Result<()> {
        let kept = matches!(decision.outcome, Outcome::Kept(_));
        match self.format {
            Format::Raw if kept => self.write_raw(&decision.record.raw)?,
            Format::Json if kept => self.write_json(decision)?,
            Format::All => self.write_json(decision)?,
            Format::Raw | Format::Json => return Ok(()),
        }
        self.stdout.flush()
    }

    fn write_raw(&mut self, raw: &[u8]) -> io::Result<()> {
        self.stdout.write_all(raw)?;
        if raw.ends_with(b"\n") {
            Ok(())
        } else {
            self.stdout.write_all(b"\n")
        }
    }

    fn write_json(&mut self, decision: &Decision) -> io::Result<()> {
        let line = JsonLine {
            position: decision.record.position,
            record: &decision.record.text,
            outcome: &decision.outcome,
        };
        serde_json::to_writer(&mut self.stdout, &line)?;
        self.stdout.write_all(b"\n")
    }
}

pub(crate) fn report(message: impl Display) {
    let _ = writeln!(io::stderr(), "jevpipe: {message}");
}

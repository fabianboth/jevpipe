use std::fmt;
use std::process::ExitCode;
use std::time::Instant;

use crate::decision::Outcome;

pub(crate) struct Summary {
    results_label: &'static str,
    records: usize,
    results: usize,
    skipped: usize,
    failed: usize,
    truncated: usize,
    cost: Option<f64>,
    started: Instant,
    closed: bool,
}

pub(crate) enum Exit {
    Success,
    NothingKept,
    Error,
}

impl Summary {
    pub(crate) fn start(results_label: &'static str) -> Self {
        Self {
            results_label,
            records: 0,
            results: 0,
            skipped: 0,
            failed: 0,
            truncated: 0,
            cost: None,
            started: Instant::now(),
            closed: false,
        }
    }

    pub(crate) fn add(&mut self, outcome: &Outcome, is_result: bool) {
        self.records += 1;
        if is_result {
            self.results += 1;
        }
        match outcome {
            Outcome::Answered { reply, truncated } => {
                if *truncated {
                    self.truncated += 1;
                }
                if let Some(cost) = reply.cost {
                    self.cost = Some(self.cost.unwrap_or_default() + cost);
                }
            }
            Outcome::Skipped(_) => self.skipped += 1,
            Outcome::Failed(_) => self.failed += 1,
        }
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
    }

    pub(crate) fn exit(&self, without_results: Exit) -> Exit {
        match (self.closed, self.failed, self.results) {
            (true, _, _) | (false, 0, 1..) => Exit::Success,
            (false, 1.., _) => Exit::Error,
            (false, 0, 0) => without_results,
        }
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} records, {} {}, {} skipped, {} failed",
            self.records, self.results, self.results_label, self.skipped, self.failed
        )?;
        if self.truncated > 0 {
            write!(formatter, ", {} truncated", self.truncated)?;
        }
        if let Some(cost) = self.cost {
            write!(formatter, ", ${}", dollars(cost))?;
        }
        write!(formatter, ", {:.1}s", self.started.elapsed().as_secs_f64())?;
        Ok(())
    }
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Success => Self::SUCCESS,
            Exit::NothingKept => Self::FAILURE,
            Exit::Error => Self::from(2),
        }
    }
}

fn dollars(amount: f64) -> String {
    let fixed = format!("{amount:.6}");
    fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
}

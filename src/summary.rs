use std::fmt;
use std::time::Instant;

use crate::decision::Outcome;
use crate::exit::Exit;
use crate::limits::{Reported, StopLine};

pub(crate) struct Summary {
    results_label: &'static str,
    records: usize,
    results: usize,
    skipped: usize,
    failed: usize,
    truncated: usize,
    reported: Reported,
    started: Instant,
    closed: bool,
    stop: Option<StopLine>,
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
            reported: Reported::default(),
            started: Instant::now(),
            closed: false,
            stop: None,
        }
    }

    pub(crate) fn add(&mut self, outcome: &Outcome) {
        self.records += 1;
        match outcome {
            Outcome::Answered { truncated, .. } => {
                if *truncated {
                    self.truncated += 1;
                }
            }
            Outcome::Skipped(_) => self.skipped += 1,
            Outcome::Failed(_) => self.failed += 1,
        }
    }

    pub(crate) fn add_result(&mut self) {
        self.results += 1;
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
    }

    pub(crate) fn finish(&mut self, reported: Reported, stop: Option<StopLine>) {
        self.reported = reported;
        self.stop = stop;
    }

    pub(crate) fn stop(&self) -> Option<&StopLine> {
        self.stop.as_ref()
    }

    pub(crate) fn exit(&self, without_results: Exit) -> Exit {
        match (self.closed, self.stop.is_some(), self.failed, self.results) {
            (true, _, _, _) | (false, false, 0, 1..) => Exit::Success,
            (false, true, _, _) => Exit::Stopped,
            (false, false, 1.., _) => Exit::Error,
            (false, false, 0, 0) => without_results,
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
        if let Some(tokens) = self.reported.tokens {
            write!(formatter, ", {} tokens", tokens.abbreviated())?;
        }
        if let Some(cost) = self.reported.cost {
            write!(formatter, ", {cost}")?;
        }
        write!(formatter, ", {:.1}s", self.started.elapsed().as_secs_f64())?;
        Ok(())
    }
}

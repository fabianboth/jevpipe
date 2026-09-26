use std::fmt;
use std::process::ExitCode;
use std::time::Instant;

use crate::decision::{Decision, Outcome};

pub(crate) struct Summary {
    records: usize,
    kept: usize,
    skipped: usize,
    failed: usize,
    cost: Option<f64>,
    model: Option<String>,
    started: Instant,
    closed: bool,
}

impl Summary {
    pub(crate) fn start() -> Self {
        Self {
            records: 0,
            kept: 0,
            skipped: 0,
            failed: 0,
            cost: None,
            model: None,
            started: Instant::now(),
            closed: false,
        }
    }

    pub(crate) fn add(&mut self, decision: &Decision) {
        self.records += 1;
        match &decision.outcome {
            Outcome::Kept(judgment) => {
                self.kept += 1;
                self.answered(judgment.cost, &judgment.model);
            }
            Outcome::Dropped(judgment) => self.answered(judgment.cost, &judgment.model),
            Outcome::Skipped { .. } => self.skipped += 1,
            Outcome::Failed { .. } => self.failed += 1,
        }
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
    }

    pub(crate) fn exit_code(&self) -> ExitCode {
        match (self.closed, self.failed, self.kept) {
            (true, _, _) | (false, 0, 1..) => ExitCode::SUCCESS,
            (false, 1.., _) => ExitCode::from(2),
            (false, 0, 0) => ExitCode::FAILURE,
        }
    }

    fn answered(&mut self, cost: Option<f64>, model: &str) {
        if let Some(cost) = cost {
            self.cost = Some(self.cost.unwrap_or_default() + cost);
        }
        self.model = Some(model.to_owned());
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} records, {} kept, {} skipped, {} failed",
            self.records, self.kept, self.skipped, self.failed
        )?;
        if let Some(cost) = self.cost {
            write!(formatter, ", ${}", dollars(cost))?;
        }
        write!(formatter, ", {:.1}s", self.started.elapsed().as_secs_f64())?;
        if let Some(model) = &self.model {
            write!(formatter, ", {model}")?;
        }
        Ok(())
    }
}

fn dollars(amount: f64) -> String {
    let fixed = format!("{amount:.6}");
    fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
}

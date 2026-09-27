use std::io::{self, Write};

use crate::decision::{Decision, Outcome};
use crate::exit::Exit;
use crate::pipeline::Command;
use crate::questions::{Questions, QuestionsError};

const QUESTION: &str = "match";

pub(crate) struct Filter {
    questions: Questions,
    threshold: f64,
}

pub(crate) fn question(value: &str) -> Result<Questions, QuestionsError> {
    Questions::yes_no(QUESTION, value)
}

impl Filter {
    pub(crate) fn new(questions: Questions, threshold: f64) -> Self {
        Self {
            questions,
            threshold,
        }
    }
}

impl Command for Filter {
    const RESULTS: &'static str = "kept";
    const WITHOUT_RESULTS: Exit = Exit::NothingKept;

    fn questions(&self) -> &Questions {
        &self.questions
    }

    fn is_result(&self, outcome: &Outcome) -> bool {
        match outcome {
            Outcome::Answered { reply, .. } => reply
                .answers
                .noul(QUESTION)
                .is_some_and(|probability| probability >= self.threshold),
            Outcome::Skipped(_) | Outcome::Failed(_) => false,
        }
    }

    fn write(&self, decision: &Decision, out: &mut impl Write) -> io::Result<()> {
        if self.is_result(&decision.outcome) {
            out.write_all(decision.record.text.as_bytes())?;
            out.write_all(decision.record.ending.terminator())?;
        }
        Ok(())
    }
}

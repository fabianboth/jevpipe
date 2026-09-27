use std::io::{self, Write};

use serde::Serialize;
use serde_json::value::RawValue;

use crate::decision::{Decision, Outcome};
use crate::pipeline::Command;
use crate::questions::Questions;
use crate::summary::Exit;

pub(crate) struct Map {
    questions: Questions,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Line<'a> {
    Answered {
        record: &'a str,
        answers: &'a RawValue,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    Unanswered {
        record: &'a str,
        outcome: &'static str,
        reason: String,
    },
}

impl Map {
    pub(crate) fn new(questions: Questions) -> Self {
        Self { questions }
    }
}

impl Command for Map {
    const RESULTS: &'static str = "answered";
    const WITHOUT_RESULTS: Exit = Exit::Success;

    fn questions(&self) -> &Questions {
        &self.questions
    }

    fn is_result(&self, outcome: &Outcome) -> bool {
        match outcome {
            Outcome::Answered { .. } => true,
            Outcome::Skipped(_) | Outcome::Failed(_) => false,
        }
    }

    fn write(&self, decision: &Decision, out: &mut impl Write) -> io::Result<()> {
        let record = decision.record.text.as_str();
        let line = match &decision.outcome {
            Outcome::Answered { reply, truncated } => Line::Answered {
                record,
                answers: reply.answers.raw(),
                truncated: *truncated,
            },
            Outcome::Skipped(reason) => Line::Unanswered {
                record,
                outcome: "skipped",
                reason: reason.to_string(),
            },
            Outcome::Failed(reason) => Line::Unanswered {
                record,
                outcome: "failed",
                reason: reason.to_string(),
            },
        };
        serde_json::to_writer(&mut *out, &line)?;
        out.write_all(b"\n")
    }
}

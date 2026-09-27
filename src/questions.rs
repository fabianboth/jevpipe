use std::fmt;
use std::fs;
use std::ops::RangeInclusive;

use serde::Deserialize;
use serde_json::value::{RawValue, to_raw_value};
use serde_json::{Value, json};

use crate::reason::Failure;

const OPTIONS: RangeInclusive<usize> = 1..=255;
const LEVELS: RangeInclusive<usize> = 2..=10;

#[derive(Clone)]
pub(crate) struct Questions {
    raw: Box<RawValue>,
    asked: Vec<(String, Kind)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Noul,
    Choice,
    Score,
}

#[derive(Deserialize)]
#[serde(expecting = "an object with type and instructions")]
struct Question {
    #[serde(rename = "type")]
    kind: Kind,
    instructions: Option<Value>,
    criteria: Option<Value>,
}

impl Questions {
    pub(crate) fn load(path: &str) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| Failure::from(error).to_string())?;
        let raw: Box<RawValue> =
            serde_json::from_str(&text).map_err(|error| format!("not JSON: {error}"))?;
        let parsed: Value =
            serde_json::from_str(raw.get()).map_err(|error| format!("not JSON: {error}"))?;
        let questions = parsed
            .as_object()
            .ok_or("must be an object of named questions")?;
        if questions.is_empty() {
            return Err("has no questions".to_owned());
        }
        let asked = questions
            .iter()
            .map(|(name, question)| {
                Question::check(question)
                    .map(|kind| (name.clone(), kind))
                    .map_err(|problem| format!("question `{name}`: {problem}"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { raw, asked })
    }

    pub(crate) fn yes_no(name: &str, instructions: &str) -> Result<Self, String> {
        let raw = to_raw_value(&json!({ name: { "type": "noul", "instructions": instructions } }))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            raw,
            asked: vec![(name.to_owned(), Kind::Noul)],
        })
    }

    pub(crate) fn raw(&self) -> &RawValue {
        &self.raw
    }

    pub(crate) fn asked(&self) -> impl Iterator<Item = (&str, Kind)> {
        self.asked.iter().map(|(name, kind)| (name.as_str(), *kind))
    }
}

impl Question {
    fn check(question: &Value) -> Result<Kind, String> {
        let question = Self::deserialize(question).map_err(|error| error.to_string())?;
        if question.instructions.is_none() {
            return Err("needs instructions".to_owned());
        }
        let criteria = question.criteria.as_ref();
        let fits = match question.kind {
            Kind::Noul => criteria.is_none_or(Value::is_object),
            Kind::Choice => criteria
                .and_then(Value::as_object)
                .is_some_and(|options| OPTIONS.contains(&options.len())),
            Kind::Score => criteria
                .and_then(Value::as_array)
                .is_some_and(|levels| LEVELS.contains(&levels.len())),
        };
        if fits {
            Ok(question.kind)
        } else {
            Err(question.kind.criteria_rule())
        }
    }
}

impl Kind {
    fn criteria_rule(self) -> String {
        match self {
            Self::Noul => "a noul's criteria must be an object".to_owned(),
            Self::Choice => format!(
                "a choice needs criteria with {} to {} options",
                OPTIONS.start(),
                OPTIONS.end()
            ),
            Self::Score => format!(
                "a score needs criteria with {} to {} levels",
                LEVELS.start(),
                LEVELS.end()
            ),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Noul => "noul",
            Self::Choice => "choice",
            Self::Score => "score",
        })
    }
}

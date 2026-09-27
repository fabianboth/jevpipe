use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::ops::RangeInclusive;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Noul,
    Choice,
    Score,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum QuestionsError {
    #[error("{0}")]
    Unreadable(Failure),
    #[error("not JSON: {0}")]
    NotJson(serde_json::Error),
    #[error("must be an object of named questions")]
    NotAnObject,
    #[error("has no questions")]
    NoQuestions,
    #[error("question `{0}` appears twice")]
    Duplicate(String),
    #[error("the question is empty")]
    EmptyQuestion,
    #[error("question `{name}`: {problem}")]
    Question { name: String, problem: Problem },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Problem {
    #[error("{0}")]
    Shape(serde_json::Error),
    #[error("needs instructions")]
    NoInstructions,
    #[error("a noul's criteria must be an object")]
    NoulCriteria,
    #[error("a choice needs criteria with {} to {} options", OPTIONS.start(), OPTIONS.end())]
    ChoiceCriteria,
    #[error("a score needs criteria with {} to {} levels", LEVELS.start(), LEVELS.end())]
    ScoreCriteria,
}

struct Entries(Vec<(String, Value)>);

struct EntriesVisitor;

#[derive(Deserialize)]
#[serde(expecting = "an object with type and instructions")]
struct Question {
    #[serde(rename = "type")]
    kind: Kind,
    instructions: Option<Value>,
    criteria: Option<Value>,
}

impl Questions {
    pub(crate) fn load(path: &str) -> Result<Self, QuestionsError> {
        let text =
            fs::read_to_string(path).map_err(|error| QuestionsError::Unreadable(error.into()))?;
        Self::parse(&text)
    }

    pub(crate) fn parse(text: &str) -> Result<Self, QuestionsError> {
        let raw: Box<RawValue> = serde_json::from_str(text).map_err(QuestionsError::NotJson)?;
        let Entries(questions) =
            serde_json::from_str(raw.get()).map_err(|_| QuestionsError::NotAnObject)?;
        if questions.is_empty() {
            return Err(QuestionsError::NoQuestions);
        }
        let mut names = HashSet::new();
        if let Some((name, _)) = questions.iter().find(|(name, _)| !names.insert(name)) {
            return Err(QuestionsError::Duplicate(name.clone()));
        }
        let asked = questions
            .iter()
            .map(|(name, question)| {
                Question::check(question)
                    .map(|kind| (name.clone(), kind))
                    .map_err(|problem| QuestionsError::Question {
                        name: name.clone(),
                        problem,
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { raw, asked })
    }

    pub(crate) fn yes_no(name: &str, instructions: &str) -> Result<Self, QuestionsError> {
        if instructions.trim().is_empty() {
            return Err(QuestionsError::EmptyQuestion);
        }
        let raw = to_raw_value(&json!({ name: { "type": "noul", "instructions": instructions } }))
            .map_err(QuestionsError::NotJson)?;
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

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(EntriesVisitor)
    }
}

impl<'de> Visitor<'de> for EntriesVisitor {
    type Value = Entries;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object of named questions")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry()? {
            entries.push(entry);
        }
        Ok(Entries(entries))
    }
}

impl Question {
    fn check(question: &Value) -> Result<Kind, Problem> {
        let question = Self::deserialize(question).map_err(Problem::Shape)?;
        if question.instructions.is_none() {
            return Err(Problem::NoInstructions);
        }
        let criteria = question.criteria.as_ref();
        match question.kind {
            Kind::Noul if criteria.is_none_or(Value::is_object) => Ok(Kind::Noul),
            Kind::Noul => Err(Problem::NoulCriteria),
            Kind::Choice
                if criteria
                    .and_then(Value::as_object)
                    .is_some_and(|options| OPTIONS.contains(&options.len())) =>
            {
                Ok(Kind::Choice)
            }
            Kind::Choice => Err(Problem::ChoiceCriteria),
            Kind::Score
                if criteria
                    .and_then(Value::as_array)
                    .is_some_and(|levels| LEVELS.contains(&levels.len())) =>
            {
                Ok(Kind::Score)
            }
            Kind::Score => Err(Problem::ScoreCriteria),
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

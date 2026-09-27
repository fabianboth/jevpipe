use std::collections::HashMap;
use std::ops::RangeInclusive;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::questions::{Kind, Questions};

pub(crate) const PROBABILITY: RangeInclusive<f64> = 0.0..=1.0;

pub(crate) struct Answers {
    raw: Box<RawValue>,
    parsed: HashMap<String, Answer>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Answer {
    Noul { noul: f64 },
    Choice {},
    Score {},
}

impl Answers {
    pub(crate) fn check(raw: Box<RawValue>, questions: &Questions) -> Result<Self, String> {
        let parsed: HashMap<String, Answer> = serde_json::from_str(raw.get())
            .map_err(|error| format!("unexpected answer: {error}"))?;
        for (name, kind) in questions.asked() {
            let answer = parsed
                .get(name)
                .ok_or_else(|| format!("unexpected answer to `{name}`: missing"))?;
            answer
                .check(kind)
                .map_err(|problem| format!("unexpected answer to `{name}`: {problem}"))?;
        }
        Ok(Self { raw, parsed })
    }

    pub(crate) fn raw(&self) -> &RawValue {
        &self.raw
    }

    pub(crate) fn noul(&self, name: &str) -> Option<f64> {
        match self.parsed.get(name) {
            Some(Answer::Noul { noul }) => Some(*noul),
            Some(Answer::Choice {} | Answer::Score {}) | None => None,
        }
    }
}

impl Answer {
    fn check(&self, asked: Kind) -> Result<(), String> {
        match (self, asked) {
            (Self::Noul { noul }, Kind::Noul) if PROBABILITY.contains(noul) => Ok(()),
            (Self::Noul { noul }, Kind::Noul) => {
                Err(format!("probability {noul} is not between 0 and 1"))
            }
            (Self::Choice {}, Kind::Choice) | (Self::Score {}, Kind::Score) => Ok(()),
            (
                Self::Noul { .. } | Self::Choice {} | Self::Score {},
                Kind::Noul | Kind::Choice | Kind::Score,
            ) => Err(format!("a {}, asked for a {asked}", self.kind())),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Noul { .. } => Kind::Noul,
            Self::Choice {} => Kind::Choice,
            Self::Score {} => Kind::Score,
        }
    }
}

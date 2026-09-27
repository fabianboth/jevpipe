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

#[derive(Debug, thiserror::Error)]
pub(crate) enum UnexpectedAnswer {
    #[error("unexpected answer: {0}")]
    Shape(#[from] serde_json::Error),
    #[error("unexpected answer to `{0}`: missing")]
    Missing(String),
    #[error("unexpected answer to `{name}`: a {answered}, asked for a {asked}")]
    Kind {
        name: String,
        answered: Kind,
        asked: Kind,
    },
    #[error("unexpected answer to `{name}`: probability {noul} is not between 0 and 1")]
    Probability { name: String, noul: f64 },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Answer {
    Noul { noul: f64 },
    Choice {},
    Score {},
}

impl Answers {
    pub(crate) fn check(
        raw: Box<RawValue>,
        questions: &Questions,
    ) -> Result<Self, UnexpectedAnswer> {
        let parsed: HashMap<String, Answer> = serde_json::from_str(raw.get())?;
        for (name, asked) in questions.asked() {
            parsed
                .get(name)
                .ok_or_else(|| UnexpectedAnswer::Missing(name.to_owned()))?
                .check(name, asked)?;
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
    fn check(&self, name: &str, asked: Kind) -> Result<(), UnexpectedAnswer> {
        match (self, asked) {
            (Self::Noul { noul }, Kind::Noul) if PROBABILITY.contains(noul) => Ok(()),
            (Self::Noul { noul }, Kind::Noul) => Err(UnexpectedAnswer::Probability {
                name: name.to_owned(),
                noul: *noul,
            }),
            (Self::Choice {}, Kind::Choice) | (Self::Score {}, Kind::Score) => Ok(()),
            (
                Self::Noul { .. } | Self::Choice {} | Self::Score {},
                Kind::Noul | Kind::Choice | Kind::Score,
            ) => Err(UnexpectedAnswer::Kind {
                name: name.to_owned(),
                answered: self.kind(),
                asked,
            }),
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

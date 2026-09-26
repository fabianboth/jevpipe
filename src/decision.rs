use std::fmt;
use std::io;

use serde::{Serialize, Serializer};

use crate::record::Record;

pub(crate) struct Decision {
    pub(crate) record: Record,
    pub(crate) outcome: Outcome,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub(crate) enum Outcome {
    Kept(Judgment),
    Dropped(Judgment),
    Skipped { reason: Skip },
    Failed { reason: Failure },
}

#[derive(Serialize)]
pub(crate) struct Judgment {
    pub(crate) probability: f64,
    pub(crate) truncated: bool,
    #[serde(skip)]
    pub(crate) cost: Option<f64>,
    #[serde(skip)]
    pub(crate) model: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Skip {
    Directory,
    Empty,
    Binary,
}

pub(crate) enum Failure {
    Unreadable(io::ErrorKind),
    NotText,
    Utf16,
    TooLarge,
    ServiceUnavailable,
}

impl Outcome {
    pub(crate) fn judged(judgment: Judgment, threshold: f64) -> Self {
        if judgment.probability >= threshold {
            Self::Kept(judgment)
        } else {
            Self::Dropped(judgment)
        }
    }

    pub(crate) fn skipped(reason: Skip) -> Self {
        Self::Skipped { reason }
    }

    pub(crate) fn failed(reason: Failure) -> Self {
        Self::Failed { reason }
    }

    pub(crate) fn unreadable(error: &io::Error) -> Self {
        Self::failed(Failure::Unreadable(error.kind()))
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(kind) if *kind == io::ErrorKind::NotFound => {
                formatter.write_str("not found")
            }
            Self::Unreadable(kind) => write!(formatter, "{kind}"),
            Self::NotText => formatter.write_str("not text"),
            Self::Utf16 => formatter.write_str("UTF-16, convert it to UTF-8"),
            Self::TooLarge => formatter.write_str("too large"),
            Self::ServiceUnavailable => formatter.write_str("service unavailable"),
        }
    }
}

impl Serialize for Failure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

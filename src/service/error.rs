use std::fmt;
use std::time::Duration;

use reqwest::Response;
use reqwest::header::RETRY_AFTER;
use serde::Deserialize;

use crate::answers::UnexpectedAnswer;

pub(crate) enum ServiceError {
    Transient { retry_after: Option<Duration> },
    TooLarge,
    Exhausted(Exhausted),
    Rejected(String),
}

#[derive(Clone, Copy)]
pub(crate) enum Exhausted {
    KeyLimit,
    Credits,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
    metadata: Option<ErrorMetadata>,
}

#[derive(Deserialize)]
struct ErrorMetadata {
    limit_source: Option<LimitSource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum LimitSource {
    OpenrouterKeyLimit,
    OpenrouterCredits,
    OpenrouterInFlightBudget,
    #[serde(other)]
    Other,
}

impl From<UnexpectedAnswer> for ServiceError {
    fn from(error: UnexpectedAnswer) -> Self {
        Self::Rejected(error.to_string())
    }
}

impl From<reqwest::Error> for ServiceError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_builder() {
            Self::Rejected(error.to_string())
        } else {
            Self::Transient { retry_after: None }
        }
    }
}

impl ServiceError {
    pub(super) async fn from_response(response: Response) -> Self {
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .map(Duration::from_secs);
        let body = response.text().await.unwrap_or_default();
        let error = serde_json::from_str::<ErrorBody>(&body).ok();
        let rejected = || {
            Self::Rejected(error.as_ref().map_or_else(
                || status.to_string(),
                |error| format!("{status}: {}", error.error.message),
            ))
        };
        match status.as_u16() {
            408 | 429 | 500 | 502 | 503 | 504 | 524 | 529 => Self::Transient { retry_after },
            402 => match error.as_ref().and_then(ErrorBody::limit_source) {
                Some(LimitSource::OpenrouterInFlightBudget) => Self::Transient { retry_after },
                Some(LimitSource::OpenrouterKeyLimit) => Self::Exhausted(Exhausted::KeyLimit),
                Some(LimitSource::OpenrouterCredits) => Self::Exhausted(Exhausted::Credits),
                Some(LimitSource::Other) | None => rejected(),
            },
            413 => Self::TooLarge,
            400 if body.contains("max_tokens_exceeded") => Self::TooLarge,
            _ => rejected(),
        }
    }

    pub(super) fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Transient { retry_after } => *retry_after,
            Self::TooLarge | Self::Exhausted(_) | Self::Rejected(_) => None,
        }
    }
}

impl ErrorBody {
    fn limit_source(&self) -> Option<&LimitSource> {
        self.error.metadata.as_ref()?.limit_source.as_ref()
    }
}

impl fmt::Display for Exhausted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::KeyLimit => "the API key's spend limit is used up",
            Self::Credits => "the OpenRouter account's credits are used up",
        })
    }
}

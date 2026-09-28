use std::fmt;
use std::time::Duration;

use reqwest::Response;
use reqwest::header::RETRY_AFTER;
use serde::Deserialize;

use crate::answers::UnexpectedAnswer;

pub(crate) enum ServiceError {
    Transient {
        cause: Unavailable,
        retry_after: Option<Duration>,
    },
    TooLarge,
    Exhausted(Exhausted),
    Rejected(String),
}

#[derive(Clone, Debug)]
pub(crate) enum Unavailable {
    Refused(String),
    TimedOut,
    ConnectionFailed,
}

#[derive(Clone, Copy)]
pub(crate) enum Exhausted {
    KeyLimit,
    Credits,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ErrorBody {
    OpenRouter { error: ErrorDetail },
    TypeSafe { detail: Detail },
}

#[derive(Deserialize)]
struct Detail {
    message: Option<String>,
    error_type: Option<String>,
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
            return Self::Rejected(error.to_string());
        }
        let cause = if error.is_timeout() {
            Unavailable::TimedOut
        } else {
            Unavailable::ConnectionFailed
        };
        Self::Transient {
            cause,
            retry_after: None,
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
        let described = || {
            error.as_ref().and_then(ErrorBody::message).map_or_else(
                || status.to_string(),
                |message| format!("{status}: {message}"),
            )
        };
        let rejected = || Self::Rejected(described());
        let transient = || Self::Transient {
            cause: Unavailable::Refused(described()),
            retry_after,
        };
        match status.as_u16() {
            408 | 429 | 500 | 502 | 503 | 504 | 524 | 529 => transient(),
            402 => match error.as_ref().and_then(ErrorBody::limit_source) {
                Some(LimitSource::OpenrouterInFlightBudget) => transient(),
                Some(LimitSource::OpenrouterKeyLimit) => Self::Exhausted(Exhausted::KeyLimit),
                Some(LimitSource::OpenrouterCredits) => Self::Exhausted(Exhausted::Credits),
                Some(LimitSource::Other) | None => rejected(),
            },
            413 => Self::TooLarge,
            400 if body.contains("max_tokens_exceeded") => Self::TooLarge,
            _ => rejected(),
        }
    }

    pub(super) fn cause(&self) -> Option<&Unavailable> {
        match self {
            Self::Transient { cause, .. } => Some(cause),
            Self::TooLarge | Self::Exhausted(_) | Self::Rejected(_) => None,
        }
    }

    pub(super) fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Transient { retry_after, .. } => *retry_after,
            Self::TooLarge | Self::Exhausted(_) | Self::Rejected(_) => None,
        }
    }
}

impl ErrorBody {
    fn message(&self) -> Option<&str> {
        match self {
            Self::OpenRouter { error } => Some(&error.message),
            Self::TypeSafe { detail } => detail.message.as_deref().or(detail.error_type.as_deref()),
        }
    }

    fn limit_source(&self) -> Option<&LimitSource> {
        match self {
            Self::OpenRouter { error } => error.metadata.as_ref()?.limit_source.as_ref(),
            Self::TypeSafe { .. } => None,
        }
    }
}

impl fmt::Display for Unavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(response) => formatter.write_str(response),
            Self::TimedOut => formatter.write_str("timed out"),
            Self::ConnectionFailed => formatter.write_str("connection failed"),
        }
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

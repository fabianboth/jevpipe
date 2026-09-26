use std::env;
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::header::RETRY_AFTER;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

const API_KEY_VARIABLE: &str = "OPENROUTER_API_KEY";
const BASE_URL_VARIABLE: &str = "JEVPIPE_BASE_URL";
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";
const ENDPOINT: &str = "/v1/systemone";
const RETRY: ExponentialBuilder = ExponentialBuilder::new()
    .with_min_delay(Duration::from_millis(500))
    .with_max_delay(Duration::from_secs(8))
    .with_max_times(4)
    .with_jitter();

pub(crate) struct ServiceConfig {
    api_key: String,
    url: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{API_KEY_VARIABLE} is not set; export your OpenRouter API key")]
pub(crate) struct MissingApiKey;

impl ServiceConfig {
    pub(crate) fn from_env() -> Result<Self, MissingApiKey> {
        let api_key = env::var(API_KEY_VARIABLE)
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or(MissingApiKey)?;
        let base_url = env::var(BASE_URL_VARIABLE).unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        Ok(Self {
            api_key,
            url: format!("{}{ENDPOINT}", base_url.trim_end_matches('/')),
        })
    }
}

pub(crate) struct Service {
    client: Client,
    config: ServiceConfig,
    model: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum State<'a> {
    Text(&'a str),
    File { path: &'a str, content: &'a str },
}

pub(crate) struct Answer {
    pub(crate) probability: f64,
    pub(crate) cost: Option<f64>,
    pub(crate) model: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ServiceError {
    #[error("service unavailable")]
    Transient { retry_after: Option<Duration> },
    #[error("too large")]
    TooLarge,
    #[error("{0}")]
    Rejected(String),
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    state: &'a State<'a>,
    questions: Questions<'a>,
}

#[derive(Serialize)]
struct Questions<'a> {
    #[serde(rename = "match")]
    matched: Question<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Question<'a> {
    Noul { instructions: &'a str },
}

#[derive(Deserialize)]
struct AnswerBody {
    model: String,
    answers: Answers,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Answers {
    #[serde(rename = "match")]
    matched: Noul,
}

#[derive(Deserialize)]
struct Noul {
    noul: f64,
}

#[derive(Deserialize)]
struct Usage {
    cost: Option<f64>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
}

impl Service {
    pub(crate) fn new(
        config: ServiceConfig,
        model: String,
        timeout: Duration,
    ) -> reqwest::Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("jevpipe/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .build()?;
        Ok(Self {
            client,
            config,
            model,
        })
    }

    pub(crate) async fn ask(
        &self,
        question: &str,
        state: &State<'_>,
    ) -> Result<Answer, ServiceError> {
        (|| self.send(question, state))
            .retry(RETRY)
            .when(|error| matches!(error, ServiceError::Transient { .. }))
            .adjust(|error, delay| delay.map(|delay| error.retry_after().unwrap_or(delay)))
            .await
    }

    async fn send(&self, question: &str, state: &State<'_>) -> Result<Answer, ServiceError> {
        let request = Request {
            model: &self.model,
            state,
            questions: Questions {
                matched: Question::Noul {
                    instructions: question,
                },
            },
        };
        let response = self
            .client
            .post(&self.config.url)
            .bearer_auth(&self.config.api_key)
            .json(&request)
            .send()
            .await?;
        if response.status().is_success() {
            Answer::from_response(response).await
        } else {
            Err(ServiceError::from_response(response).await)
        }
    }
}

impl Answer {
    async fn from_response(response: Response) -> Result<Self, ServiceError> {
        let body = response.bytes().await?;
        let body: AnswerBody = serde_json::from_slice(&body)
            .map_err(|error| ServiceError::Rejected(format!("unexpected answer: {error}")))?;
        Ok(Self {
            probability: body.answers.matched.noul,
            cost: body.usage.and_then(|usage| usage.cost),
            model: body.model,
        })
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
    async fn from_response(response: Response) -> Self {
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .map(Duration::from_secs);
        let body = response.text().await.unwrap_or_default();
        match status.as_u16() {
            408 | 429 | 500 | 502 | 503 | 504 | 524 | 529 => Self::Transient { retry_after },
            413 => Self::TooLarge,
            400 if body.contains("max_tokens_exceeded") => Self::TooLarge,
            _ => Self::Rejected(rejection_message(status, &body)),
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Transient { retry_after } => *retry_after,
            Self::TooLarge | Self::Rejected(_) => None,
        }
    }
}

fn rejection_message(status: StatusCode, body: &str) -> String {
    serde_json::from_str::<ErrorBody>(body).map_or_else(
        |_| status.to_string(),
        |body| format!("{status}: {}", body.error.message),
    )
}

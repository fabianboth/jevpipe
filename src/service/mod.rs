mod error;

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::answers::{Answers, UnexpectedAnswer};
use crate::auth::ApiKey;
use crate::cost::Cost;
use crate::questions::Questions;
use crate::settings::Settings;
use crate::tokens::Tokens;

pub(crate) use error::{Exhausted, ServiceError};

const ENDPOINT: &str = "/v1/systemone";
const RETRY: ExponentialBuilder = ExponentialBuilder::new()
    .with_min_delay(Duration::from_millis(500))
    .with_max_delay(Duration::from_secs(8))
    .with_max_times(4)
    .with_jitter();

pub(crate) struct ServiceConfig {
    api_key: ApiKey,
    url: String,
}

impl ServiceConfig {
    pub(crate) fn new(base_url: &str, api_key: ApiKey) -> Self {
        Self {
            api_key,
            url: format!("{}{ENDPOINT}", base_url.trim_end_matches('/')),
        }
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

pub(crate) struct Reply {
    pub(crate) answers: Answers,
    pub(crate) usage: Usage,
}

pub(crate) struct Usage {
    pub(crate) tokens: Option<Tokens>,
    pub(crate) cost: Option<Cost>,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    state: &'a State<'a>,
    questions: &'a RawValue,
}

#[derive(Deserialize)]
struct ReplyBody {
    answers: Box<RawValue>,
    #[serde(default)]
    usage: UsageBody,
}

#[derive(Default, Deserialize)]
struct UsageBody {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost: Option<f64>,
}

impl Service {
    pub(crate) fn new(config: ServiceConfig, settings: &Settings) -> reqwest::Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("jevpipe/", env!("CARGO_PKG_VERSION")))
            .timeout(settings.request_timeout)
            .build()?;
        Ok(Self {
            client,
            config,
            model: settings.model.clone(),
        })
    }

    pub(crate) async fn ask(
        &self,
        questions: &Questions,
        state: &State<'_>,
    ) -> Result<Reply, ServiceError> {
        (|| self.send(questions, state))
            .retry(RETRY)
            .when(|error| matches!(error, ServiceError::Transient { .. }))
            .adjust(|error, delay| delay.map(|delay| error.retry_after().unwrap_or(delay)))
            .await
    }

    async fn send(&self, questions: &Questions, state: &State<'_>) -> Result<Reply, ServiceError> {
        let request = Request {
            model: &self.model,
            state,
            questions: questions.raw(),
        };
        let response = self
            .client
            .post(&self.config.url)
            .bearer_auth(self.config.api_key.expose())
            .json(&request)
            .send()
            .await?;
        if response.status().is_success() {
            Reply::from_response(response, questions).await
        } else {
            Err(ServiceError::from_response(response).await)
        }
    }
}

impl Reply {
    async fn from_response(
        response: Response,
        questions: &Questions,
    ) -> Result<Self, ServiceError> {
        let body = response.bytes().await?;
        let body: ReplyBody = serde_json::from_slice(&body).map_err(UnexpectedAnswer::from)?;
        Ok(Self {
            answers: Answers::check(body.answers, questions)?,
            usage: body.usage.try_into()?,
        })
    }
}

impl TryFrom<UsageBody> for Usage {
    type Error = ServiceError;

    fn try_from(body: UsageBody) -> Result<Self, Self::Error> {
        Ok(Self {
            tokens: body
                .input_tokens
                .zip(body.output_tokens)
                .map(|(input, output)| Tokens::from_count(input.saturating_add(output))),
            cost: body
                .cost
                .map(Cost::from_dollars)
                .transpose()
                .map_err(|error| ServiceError::Rejected(format!("unexpected cost: {error}")))?,
        })
    }
}

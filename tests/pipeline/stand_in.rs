use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{Map, Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::fixture::QUESTIONS;

const TYPESAFE_MODELS: [&str; 2] = ["jev-latest", "jev-1.13.0"];

pub(crate) const OPENROUTER_KEY: &str = "test-key";
pub(crate) const TYPESAFE_KEY: &str = "test-typesafe-key";
const KEY_VARIABLES: [&str; 2] = ["OPENROUTER_API_KEY", "TYPESAFE_API_KEY"];

pub(crate) struct StandIn {
    server: MockServer,
    config_dir: TempDir,
    provider: Provider,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Provider {
    #[default]
    OpenRouter,
    TypeSafe,
}

impl StandIn {
    pub(crate) async fn start() -> Self {
        Self::start_as(Provider::OpenRouter).await
    }

    pub(crate) async fn typesafe() -> Self {
        Self::start_as(Provider::TypeSafe).await
    }

    async fn start_as(provider: Provider) -> Self {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/systemone"))
            .respond_with(Responder {
                attempts: Mutex::default(),
                provider,
            })
            .mount(&server)
            .await;
        let config_dir = TempDir::new().unwrap();
        let stand_in = Self {
            server,
            config_dir,
            provider,
        };
        stand_in.configure("");
        stand_in
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.config_dir.path().join("config.toml")
    }

    pub(crate) fn configure(&self, settings: &str) {
        let base_url = format!("base-url = \"{}\"\n", self.server.uri());
        let provider = match self.provider {
            Provider::OpenRouter => "",
            Provider::TypeSafe => "provider = \"typesafe\"\n",
        };
        fs::write(self.config(), base_url + provider + settings).unwrap();
    }

    pub(crate) async fn keys(&self) -> Vec<String> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|request| {
                let header = request.headers.get("authorization").unwrap();
                header.to_str().unwrap().to_owned()
            })
            .collect()
    }

    pub(crate) async fn requests(&self) -> Vec<Value> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|request| request.body_json().unwrap())
            .collect()
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jevpipe"));
        for variable in KEY_VARIABLES {
            command.env_remove(variable);
        }
        let (variable, key) = match self.provider {
            Provider::OpenRouter => ("OPENROUTER_API_KEY", OPENROUTER_KEY),
            Provider::TypeSafe => ("TYPESAFE_API_KEY", TYPESAFE_KEY),
        };
        command
            .env("JEVPIPE_CONFIG", self.config())
            .env(variable, key)
            .env("NO_PROXY", "127.0.0.1");
        command
    }

    pub(crate) fn jevpipe(&self) -> assert_cmd::Command {
        assert_cmd::Command::from_std(self.command())
    }

    pub(crate) fn filter(&self) -> assert_cmd::Command {
        let mut command = self.jevpipe();
        command.args(["filter", "Is it?"]);
        command
    }

    pub(crate) fn map(&self) -> assert_cmd::Command {
        let mut command = self.jevpipe();
        command.args(["map", "-q", QUESTIONS]);
        command
    }
}

struct Responder {
    attempts: Mutex<HashMap<String, usize>>,
    provider: Provider,
}

impl Respond for Responder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = request.body_json().unwrap();
        let state = body["state"]
            .as_str()
            .or_else(|| body["state"]["content"].as_str())
            .unwrap()
            .to_owned();
        let attempt = {
            let mut attempts = self.attempts.lock().unwrap();
            let count = attempts.entry(state.clone()).or_default();
            *count += 1;
            *count
        };
        let markers = Markers::parse(&state);
        let response = markers.respond(&body, attempt, self.provider);
        match markers.slow {
            Some(millis) if attempt == 1 => response.set_delay(Duration::from_millis(millis)),
            Some(_) | None => response,
        }
    }
}

#[derive(Default)]
struct Markers {
    probability: Option<f64>,
    choice: Option<String>,
    level: Option<usize>,
    fail: Option<(u16, usize)>,
    status: Option<u16>,
    too_large: bool,
    fits: Option<usize>,
    malformed: bool,
    wrong_type: bool,
    slow: Option<u64>,
    retry_after: Option<u64>,
    cost: Cost,
    tokens: Tokens,
    detail: Option<Detail>,
    limit: Option<String>,
    in_flight: Option<usize>,
}

enum Detail {
    ErrorTypeOnly,
    Validation,
}

#[derive(Default)]
enum Tokens {
    #[default]
    Usual,
    Of(u64),
    Missing,
}

#[derive(Default)]
enum Cost {
    #[default]
    Usual,
    Of(f64),
    Missing,
}

impl Markers {
    fn parse(state: &str) -> Self {
        let mut markers = Self::default();
        for word in state.split_whitespace() {
            let word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            match word.split_once('=') {
                Some(("p", value)) => markers.probability = Some(value.parse().unwrap()),
                Some(("choice", value)) => markers.choice = Some(value.to_owned()),
                Some(("level", value)) => markers.level = Some(value.parse().unwrap()),
                Some(("fail", value)) => {
                    let (status, times) = value.split_once('x').unwrap();
                    markers.fail = Some((status.parse().unwrap(), times.parse().unwrap()));
                }
                Some(("status", value)) => markers.status = Some(value.parse().unwrap()),
                Some(("slow", value)) => markers.slow = Some(value.parse().unwrap()),
                Some(("after", value)) => markers.retry_after = Some(value.parse().unwrap()),
                Some(("cost", value)) => markers.cost = Cost::Of(value.parse().unwrap()),
                Some(("tokens", value)) => markers.tokens = Tokens::Of(value.parse().unwrap()),
                Some(("limit", value)) => markers.limit = Some(format!("openrouter_{value}")),
                Some(("inflight", value)) => markers.in_flight = Some(value.parse().unwrap()),
                Some(("fits", value)) => markers.fits = Some(value.parse().unwrap()),
                None if word == "toolarge" => markers.too_large = true,
                None if word == "malformed" => markers.malformed = true,
                None if word == "wrongtype" => markers.wrong_type = true,
                None if word == "nocost" => markers.cost = Cost::Missing,
                None if word == "notokens" => markers.tokens = Tokens::Missing,
                None if word == "errortype" => markers.detail = Some(Detail::ErrorTypeOnly),
                None if word == "validation" => markers.detail = Some(Detail::Validation),
                _ => {}
            }
        }
        if let Some(characters) = markers.fits {
            markers.too_large |= state.chars().count() > characters;
        }
        markers
    }

    fn respond(&self, body: &Value, attempt: usize, provider: Provider) -> ResponseTemplate {
        let error = |status, message| error(provider, status, message);
        if let Some(status) = self.status {
            return error(status, "No cookie auth credentials found");
        }
        if provider == Provider::TypeSafe {
            if let Some(model) = body["model"]
                .as_str()
                .filter(|model| !TYPESAFE_MODELS.contains(model))
            {
                return error(400, &format!("Unknown model: {model}"));
            }
            match self.detail {
                Some(Detail::ErrorTypeOnly) => {
                    return ResponseTemplate::new(400)
                        .set_body_json(json!({ "detail": { "error_type": "api_usage_error" } }));
                }
                Some(Detail::Validation) => {
                    return ResponseTemplate::new(422).set_body_json(json!({ "detail": [
                        { "type": "missing", "loc": ["body", "questions"], "msg": "Field required" }
                    ] }));
                }
                None => {}
            }
        }
        if let Some(source) = &self.limit {
            return payment_required(source);
        }
        if let Some(times) = self.in_flight
            && attempt <= times
        {
            return payment_required("openrouter_in_flight_budget")
                .insert_header("Retry-After", "0");
        }
        if self.malformed {
            return ResponseTemplate::new(200)
                .set_body_json(json!({ "model": "typesafe/jev-test", "answers": {} }));
        }
        if self.too_large {
            return error(
                400,
                "HTTP 400: {\"detail\":{\"error_type\":\"max_tokens_exceeded\"}}",
            );
        }
        if let Some((status, times)) = self.fail
            && attempt <= times
        {
            let retry_after = self.retry_after.unwrap_or(0).to_string();
            return error(status, "Provider returned error")
                .insert_header("Retry-After", retry_after);
        }
        let mut usage = match self.tokens {
            Tokens::Usual => json!({ "input_tokens": 310, "output_tokens": 20 }),
            Tokens::Of(count) => json!({ "input_tokens": count, "output_tokens": 0 }),
            Tokens::Missing => json!({}),
        };
        match (provider, &self.cost) {
            (Provider::OpenRouter, Cost::Usual) => usage["cost"] = json!(0.00001),
            (Provider::OpenRouter, Cost::Of(cost)) => usage["cost"] = json!(cost),
            (Provider::OpenRouter, Cost::Missing) | (Provider::TypeSafe, _) => {}
        }
        ResponseTemplate::new(200).set_body_json(json!({
            "model": "typesafe/jev-test",
            "answers": self.answers(&body["questions"]),
            "usage": usage,
            "id": "gen-dec-test",
            "provider": "TypeSafe"
        }))
    }
}

impl Markers {
    fn answers(&self, questions: &Value) -> Map<String, Value> {
        questions
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, question)| (name.clone(), self.answer(question)))
            .collect()
    }

    fn answer(&self, question: &Value) -> Value {
        match question["type"].as_str() {
            Some("choice") if !self.wrong_type => {
                self.choice(question["criteria"].as_object().unwrap())
            }
            Some("score") if !self.wrong_type => {
                self.score(question["criteria"].as_array().unwrap())
            }
            Some(_) | None => json!({ "type": "noul", "noul": self.probability.unwrap_or(0.1) }),
        }
    }

    fn choice(&self, options: &Map<String, Value>) -> Value {
        let first = options.keys().next().unwrap();
        let chosen = self.choice.as_ref().unwrap_or(first);
        let probabilities: Map<String, Value> = options
            .keys()
            .map(|option| (option.clone(), json!(u8::from(option == chosen))))
            .collect();
        json!({
            "type": "choice",
            "choice": chosen,
            "probabilities": probabilities,
            "confidence": 1
        })
    }

    fn score(&self, levels: &[Value]) -> Value {
        let chosen = self.level.unwrap_or(0);
        let legend: Map<String, Value> = levels
            .iter()
            .enumerate()
            .map(|(level, text)| (level.to_string(), text.clone()))
            .collect();
        let probabilities: Map<String, Value> = (0..levels.len())
            .map(|level| (level.to_string(), json!(u8::from(level == chosen))))
            .collect();
        json!({
            "type": "score",
            "score": chosen,
            "legend": legend,
            "probabilities": probabilities,
            "confidence": 1
        })
    }
}

fn error(provider: Provider, status: u16, message: &str) -> ResponseTemplate {
    let body = match provider {
        Provider::OpenRouter => json!({ "error": { "message": message, "code": status } }),
        Provider::TypeSafe => {
            json!({ "detail": { "error_type": "api_usage_error", "message": message } })
        }
    };
    ResponseTemplate::new(status).set_body_json(body)
}

fn payment_required(limit_source: &str) -> ResponseTemplate {
    ResponseTemplate::new(402).set_body_json(json!({
        "error": {
            "code": 402,
            "message": "Payment required",
            "metadata": { "reason": "limit", "limit_source": limit_source }
        }
    }))
}

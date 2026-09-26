use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

pub(crate) struct StandIn {
    server: MockServer,
}

impl StandIn {
    pub(crate) async fn start() -> Self {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/systemone"))
            .respond_with(Responder::default())
            .mount(&server)
            .await;
        Self { server }
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
        command
            .env("JEVPIPE_BASE_URL", self.server.uri())
            .env("OPENROUTER_API_KEY", "test-key")
            .env("NO_PROXY", "127.0.0.1");
        command
    }

    pub(crate) fn jevpipe(&self) -> assert_cmd::Command {
        assert_cmd::Command::from_std(self.command())
    }
}

#[derive(Default)]
struct Responder {
    attempts: Mutex<HashMap<String, usize>>,
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
        let response = markers.answer(attempt);
        match markers.slow {
            Some(millis) if attempt == 1 => response.set_delay(Duration::from_millis(millis)),
            Some(_) | None => response,
        }
    }
}

#[derive(Default)]
struct Markers {
    probability: Option<f64>,
    fail: Option<(u16, usize)>,
    fatal: Option<u16>,
    too_large: bool,
    slow: Option<u64>,
}

impl Markers {
    fn parse(state: &str) -> Self {
        let mut markers = Self::default();
        for word in state.split_whitespace() {
            let word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            match word.split_once('=') {
                Some(("p", value)) => markers.probability = Some(value.parse().unwrap()),
                Some(("fail", value)) => {
                    let (status, times) = value.split_once('x').unwrap();
                    markers.fail = Some((status.parse().unwrap(), times.parse().unwrap()));
                }
                Some(("fatal", value)) => markers.fatal = Some(value.parse().unwrap()),
                Some(("slow", value)) => markers.slow = Some(value.parse().unwrap()),
                None if word == "toolarge" => markers.too_large = true,
                _ => {}
            }
        }
        markers
    }

    fn answer(&self, attempt: usize) -> ResponseTemplate {
        if let Some(status) = self.fatal {
            return error(status, "No cookie auth credentials found");
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
            return error(status, "Provider returned error").insert_header("Retry-After", "0");
        }
        ResponseTemplate::new(200).set_body_json(json!({
            "model": "typesafe/jev-test",
            "answers": { "match": { "type": "noul", "noul": self.probability.unwrap_or(0.1) } },
            "usage": { "input_tokens": 310, "output_tokens": 20, "cost": 0.00001 },
            "id": "gen-dec-test",
            "provider": "TypeSafe"
        }))
    }
}

fn error(status: u16, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .set_body_json(json!({ "error": { "message": message, "code": status } }))
}

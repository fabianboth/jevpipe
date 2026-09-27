use reqwest::Url;

use crate::settings;

pub(super) const BASE_URL: &str = "base-url";
pub(super) const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";

#[derive(Debug, thiserror::Error)]
pub(crate) enum KeyError {
    #[error("unknown key `{0}`; the keys are {keys}", keys = all().join(", "))]
    Unknown(String),
    #[error("`{0}` needs a single value")]
    NotSingle(String),
    #[error("invalid value '{value}' for `{key}`: {reason}")]
    Invalid {
        key: String,
        value: String,
        reason: String,
    },
}

pub(super) fn all() -> Vec<String> {
    let mut keys = settings::keys();
    keys.push(BASE_URL.to_owned());
    keys.sort();
    keys
}

pub(super) fn is_known(key: &str) -> bool {
    all().iter().any(|name| name == key)
}

pub(super) fn check(key: &str, value: &str) -> Result<(), KeyError> {
    let invalid = |reason| KeyError::Invalid {
        key: key.to_owned(),
        value: value.to_owned(),
        reason,
    };
    if key == BASE_URL {
        base_url(value).map_err(invalid)
    } else if is_known(key) {
        settings::check(key, value).map_err(invalid)
    } else {
        Err(KeyError::Unknown(key.to_owned()))
    }
}

pub(super) fn default(key: &str) -> Option<String> {
    if key == BASE_URL {
        Some(DEFAULT_BASE_URL.to_owned())
    } else {
        settings::default(key)
    }
}

fn base_url(value: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|error| error.to_string())?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        _ => Err("not an http or https URL".to_owned()),
    }
}

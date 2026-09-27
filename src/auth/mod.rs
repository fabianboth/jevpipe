mod command;
mod keychain;
mod prompt;
#[cfg(target_os = "macos")]
mod security;

use std::env;
use std::fmt;

pub(crate) use command::run;
use keychain::Unavailable;

const API_KEY_VARIABLE: &str = "OPENROUTER_API_KEY";

pub(crate) struct ApiKey(String);

pub(crate) enum Source {
    Environment,
    Keychain,
    NotSet,
    Unavailable(Unavailable),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LookupError {
    #[error("no API key: set {API_KEY_VARIABLE} or run jevpipe auth set-key")]
    Missing,
    #[error("{0}; set {API_KEY_VARIABLE} instead")]
    Keychain(#[from] Unavailable),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
    #[error("no API key given")]
    Empty,
    #[error(
        "that does not look like an API key (printable ASCII characters only, no spaces, quotes or backslashes)"
    )]
    NotAKey,
    #[error("{0}; set {API_KEY_VARIABLE} instead")]
    Keychain(#[from] Unavailable),
}

impl ApiKey {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

pub(crate) fn api_key() -> Result<ApiKey, LookupError> {
    lookup(env::var(API_KEY_VARIABLE).ok()).map(|(key, _)| key)
}

pub(crate) fn source() -> Source {
    match lookup(env::var(API_KEY_VARIABLE).ok()) {
        Ok((_, source)) => source,
        Err(LookupError::Missing) => Source::NotSet,
        Err(LookupError::Keychain(unavailable)) => Source::Unavailable(unavailable),
    }
}

fn lookup(variable: Option<String>) -> Result<(ApiKey, Source), LookupError> {
    if let Some(key) = variable.filter(|key| !key.is_empty()) {
        return Ok((ApiKey(key), Source::Environment));
    }
    match keychain::get()? {
        Some(key) => Ok((ApiKey(key), Source::Keychain)),
        None => Err(LookupError::Missing),
    }
}

fn store(line: &str) -> Result<(), StoreError> {
    let key = line.trim();
    if key.is_empty() {
        return Err(StoreError::Empty);
    }
    let allowed = |byte: u8| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\'' | b'\\');
    if !key.bytes().all(allowed) {
        return Err(StoreError::NotAKey);
    }
    Ok(keychain::set(key)?)
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment => write!(formatter, "from {API_KEY_VARIABLE}"),
            Self::Keychain => formatter.write_str("from the keychain"),
            Self::NotSet => formatter.write_str("not set"),
            Self::Unavailable(unavailable) => {
                write!(formatter, "keychain unavailable: {}", unavailable.reason())
            }
        }
    }
}

#[cfg(test)]
mod tests;

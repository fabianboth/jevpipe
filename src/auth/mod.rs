mod command;
mod keychain;
mod prompt;
#[cfg(target_os = "macos")]
mod security;

use std::env;
use std::fmt;

pub(crate) use command::run;
use keychain::Unavailable;

use crate::provider::Provider;

pub(crate) struct ApiKey(String);

pub(crate) enum Source {
    Environment(Provider),
    Keychain,
    NotSet,
    Unavailable(Unavailable),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LookupError {
    #[error(
        "no {name} API key: set {variable} or run jevpipe auth set-key (provider {0}; jevpipe config set provider {other} switches)",
        name = .0.name(),
        variable = .0.key_variable(),
        other = .0.other()
    )]
    Missing(Provider),
    #[error(transparent)]
    Keychain(NoKeychain),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
    #[error("no API key given")]
    Empty,
    #[error(
        "that does not look like an API key (printable ASCII characters only, no spaces, quotes or backslashes)"
    )]
    NotAKey,
    #[error(transparent)]
    Keychain(NoKeychain),
}

#[derive(Debug, thiserror::Error)]
#[error("{unavailable}; set {variable} instead", variable = .provider.key_variable())]
pub(crate) struct NoKeychain {
    unavailable: Unavailable,
    provider: Provider,
}

impl ApiKey {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

pub(crate) fn api_key(provider: Provider) -> Result<ApiKey, LookupError> {
    lookup(provider, variable(provider)).map(|(key, _)| key)
}

pub(crate) fn source(provider: Provider) -> Source {
    match lookup(provider, variable(provider)) {
        Ok((_, source)) => source,
        Err(LookupError::Missing(_)) => Source::NotSet,
        Err(LookupError::Keychain(error)) => Source::Unavailable(error.unavailable),
    }
}

fn variable(provider: Provider) -> Option<String> {
    env::var(provider.key_variable()).ok()
}

fn lookup(provider: Provider, variable: Option<String>) -> Result<(ApiKey, Source), LookupError> {
    if let Some(key) = variable.filter(|key| !key.is_empty()) {
        return Ok((ApiKey(key), Source::Environment(provider)));
    }
    match keychain::get(provider) {
        Ok(Some(key)) => Ok((ApiKey(key), Source::Keychain)),
        Ok(None) => Err(LookupError::Missing(provider)),
        Err(unavailable) => Err(LookupError::Keychain(NoKeychain {
            unavailable,
            provider,
        })),
    }
}

fn store(provider: Provider, line: &str) -> Result<(), StoreError> {
    let key = line.trim();
    if key.is_empty() {
        return Err(StoreError::Empty);
    }
    let allowed = |byte: u8| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\'' | b'\\');
    if !key.bytes().all(allowed) {
        return Err(StoreError::NotAKey);
    }
    keychain::set(provider, key).map_err(|unavailable| {
        StoreError::Keychain(NoKeychain {
            unavailable,
            provider,
        })
    })
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment(provider) => write!(formatter, "from {}", provider.key_variable()),
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

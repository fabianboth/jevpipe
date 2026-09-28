use std::fmt;

use reqwest::Url;

use crate::provider::Provider;
use crate::settings;

pub(super) const BASE_URL: &str = "base-url";
pub(super) const PROVIDER: &str = "provider";
pub(super) const MAX_COST: &str = "max-cost";

#[derive(Debug, thiserror::Error)]
pub(crate) enum KeyError {
    #[error(
        "unknown key `{0}`; the keys are {keys}, and all but {PROVIDER} also as {sections}",
        keys = all().join(", "),
        sections = sections()
    )]
    Unknown(String),
    #[error("`{0}` needs a single value")]
    NotSingle(String),
    #[error("`{0}` must be a section of settings")]
    NotASection(String),
    #[error("invalid value '{value}' for `{key}`: {reason}")]
    Invalid {
        key: String,
        value: String,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    Every,
    Only(Provider),
}

#[derive(Clone, Copy)]
pub(super) struct ScopedKey<'a> {
    pub(super) scope: Scope,
    pub(super) name: &'a str,
}

impl<'a> ScopedKey<'a> {
    pub(super) fn parse(text: &'a str) -> Self {
        let every = Self {
            scope: Scope::Every,
            name: text,
        };
        let Some((prefix, name)) = text.split_once('.') else {
            return every;
        };
        match prefix.parse() {
            Ok(provider) => Self {
                scope: Scope::Only(provider),
                name,
            },
            Err(_) => every,
        }
    }

    pub(super) fn is_known(self) -> bool {
        all().iter().any(|name| name == self.name)
            && (self.scope == Scope::Every || self.name != PROVIDER)
    }
}

impl fmt::Display for ScopedKey<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.scope {
            Scope::Every => formatter.write_str(self.name),
            Scope::Only(provider) => write!(formatter, "{provider}.{}", self.name),
        }
    }
}

pub(super) fn all() -> Vec<String> {
    let mut keys = settings::keys();
    keys.extend([BASE_URL, PROVIDER].map(str::to_owned));
    keys.sort();
    keys
}

pub(super) fn is_setting(key: &str) -> bool {
    key != BASE_URL && key != PROVIDER
}

pub(super) fn check(key: ScopedKey<'_>, value: &str) -> Result<(), KeyError> {
    let invalid = |reason| KeyError::Invalid {
        key: key.to_string(),
        value: value.to_owned(),
        reason,
    };
    if !key.is_known() {
        return Err(KeyError::Unknown(key.to_string()));
    }
    if key.name == BASE_URL {
        base_url(value).map_err(invalid)
    } else if key.name == PROVIDER {
        value
            .parse::<Provider>()
            .map(drop)
            .map_err(|error| invalid(error.to_string()))
    } else if let Scope::Only(provider) = key.scope
        && key.name == MAX_COST
        && !provider.reports_cost()
    {
        Err(invalid(format!(
            "{} reports no cost; use {provider}.max-tokens",
            provider.name()
        )))
    } else {
        settings::check(key.name, value).map_err(invalid)
    }
}

pub(super) fn default(key: &str, provider: Provider) -> Option<String> {
    if key == BASE_URL {
        Some(provider.default_base_url().to_owned())
    } else if key == PROVIDER {
        Some(Provider::default().to_string())
    } else {
        settings::default(key)
    }
}

fn sections() -> String {
    Provider::ALL
        .map(|provider| format!("{provider}.<key>"))
        .join(" or ")
}

fn base_url(value: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|error| error.to_string())?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        _ => Err("not an http or https URL".to_owned()),
    }
}

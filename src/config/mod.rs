mod command;
mod file;
mod keys;

use std::fmt;
use std::io;
use std::path::PathBuf;

pub(crate) use command::run;
use keys::{BASE_URL, KeyError, MAX_COST, PROVIDER, Scope, ScopedKey};

use crate::provider::Provider;

pub(crate) fn key_names() -> String {
    keys::all().join(", ")
}

pub(crate) struct Config {
    entries: Vec<Entry>,
}

struct Entry {
    scope: Scope,
    name: String,
    value: String,
}

pub(crate) struct Setting {
    pub(crate) value: String,
    pub(crate) origin: Origin,
}

#[derive(Clone, Copy)]
pub(crate) enum Origin {
    ConfigFile(Scope),
    Default,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("cannot find the user config directory: {0}")]
    NoHome(#[from] etcetera::HomeDirError),
    #[error("{}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("{}: {source}", path.display())]
    Syntax {
        path: PathBuf,
        source: toml_edit::TomlError,
    },
    #[error("{}: {source}", path.display())]
    Key { path: PathBuf, source: KeyError },
    #[error(transparent)]
    Rejected(#[from] KeyError),
    #[error("cannot write output: {0}")]
    Output(io::Error),
}

impl Config {
    pub(crate) fn load() -> Result<Self, ConfigError> {
        let path = file::path()?;
        let document = file::read(&path)?;
        let entries = file::entries(&document).map_err(|source| ConfigError::Key {
            path: path.clone(),
            source,
        })?;
        Ok(Self { entries })
    }

    pub(crate) fn provider(&self) -> Provider {
        self.find(Scope::Every, PROVIDER)
            .and_then(|entry| entry.value.parse().ok())
            .unwrap_or_default()
    }

    pub(crate) fn base_url(&self) -> &str {
        let provider = self.provider();
        self.effective(provider, BASE_URL)
            .map_or_else(|| provider.default_base_url(), |entry| &entry.value)
    }

    pub(crate) fn shared_spend_limit(&self) -> Option<&str> {
        self.find(Scope::Every, MAX_COST)
            .map(|entry| entry.value.as_str())
    }

    pub(crate) fn settings(&self) -> impl Iterator<Item = (&str, &str)> {
        let active = self.provider();
        self.entries
            .iter()
            .filter(move |entry| keys::is_setting(&entry.name) && self.applies(entry, active))
            .map(|entry| (entry.name.as_str(), entry.value.as_str()))
    }

    pub(crate) fn setting(&self, text: &str) -> Result<Setting, KeyError> {
        let key = ScopedKey::parse(text);
        if !key.is_known() {
            return Err(KeyError::Unknown(text.to_owned()));
        }
        let provider = match key.scope {
            Scope::Every => self.provider(),
            Scope::Only(provider) => provider,
        };
        match (
            self.effective(provider, key.name),
            keys::default(key.name, provider),
        ) {
            (Some(entry), _) => Ok(Setting {
                value: entry.value.clone(),
                origin: Origin::ConfigFile(entry.scope),
            }),
            (None, Some(value)) => Ok(Setting {
                value,
                origin: Origin::Default,
            }),
            (None, None) => Err(KeyError::Unknown(text.to_owned())),
        }
    }

    fn applies(&self, entry: &Entry, active: Provider) -> bool {
        match entry.scope {
            Scope::Only(provider) => provider == active,
            Scope::Every => self.find(Scope::Only(active), &entry.name).is_none(),
        }
    }

    fn effective(&self, provider: Provider, name: &str) -> Option<&Entry> {
        self.find(Scope::Only(provider), name)
            .or_else(|| self.find(Scope::Every, name))
    }

    fn find(&self, scope: Scope, name: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.scope == scope && entry.name == name)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigFile(Scope::Every) => formatter.write_str("config file"),
            Self::ConfigFile(Scope::Only(provider)) => {
                write!(formatter, "config file [{provider}]")
            }
            Self::Default => formatter.write_str("default"),
        }
    }
}

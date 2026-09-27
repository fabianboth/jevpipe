mod command;
mod file;
mod keys;

use std::fmt;
use std::io;
use std::path::PathBuf;

pub(crate) use command::run;
use keys::{BASE_URL, DEFAULT_BASE_URL, KeyError};

pub(crate) fn key_names() -> String {
    keys::all().join(", ")
}

pub(crate) struct Config {
    values: Vec<(String, String)>,
}

pub(crate) struct Setting {
    pub(crate) value: String,
    pub(crate) origin: Origin,
}

#[derive(Clone, Copy)]
pub(crate) enum Origin {
    ConfigFile,
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
        let key_error = |source| ConfigError::Key {
            path: path.clone(),
            source,
        };
        let values = file::read(&path)?
            .iter()
            .map(|(key, item)| {
                let value = file::text(item)
                    .ok_or_else(|| key_error(KeyError::NotSingle(key.to_owned())))?;
                keys::check(key, &value).map_err(key_error)?;
                Ok((key.to_owned(), value))
            })
            .collect::<Result<_, ConfigError>>()?;
        Ok(Self { values })
    }

    pub(crate) fn base_url(&self) -> &str {
        self.get(BASE_URL).unwrap_or(DEFAULT_BASE_URL)
    }

    pub(crate) fn settings(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .filter(|(key, _)| *key != BASE_URL)
    }

    pub(crate) fn setting(&self, key: &str) -> Result<Setting, KeyError> {
        match (self.get(key), keys::default(key)) {
            (Some(value), _) => Ok(Setting {
                value: value.to_owned(),
                origin: Origin::ConfigFile,
            }),
            (None, Some(value)) => Ok(Setting {
                value,
                origin: Origin::Default,
            }),
            (None, None) => Err(KeyError::Unknown(key.to_owned())),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConfigFile => "config file",
            Self::Default => "default",
        })
    }
}

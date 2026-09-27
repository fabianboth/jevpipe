use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use toml_edit::{DocumentMut, Item, Value};

use super::ConfigError;
use super::keys::{self, KeyError};

const PATH_VARIABLE: &str = "JEVPIPE_CONFIG";

pub(super) fn path() -> Result<PathBuf, ConfigError> {
    match env::var_os(PATH_VARIABLE).filter(|path| !path.is_empty()) {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(etcetera::choose_base_strategy()?
            .config_dir()
            .join("jevpipe")
            .join("config.toml")),
    }
}

pub(super) fn set(key: &str, value: &str) -> Result<(), ConfigError> {
    keys::check(key, value)?;
    let path = path()?;
    let mut document = read(&path)?;
    let mut new = toml(value);
    match document.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            *new.decor_mut() = old.decor().clone();
            *old = new;
        }
        None => {
            document.insert(key, Item::Value(new));
        }
    }
    write(&path, &document)
}

pub(super) fn unset(key: &str) -> Result<(), ConfigError> {
    let path = path()?;
    let mut document = read(&path)?;
    match document.remove(key) {
        Some(_) => write(&path, &document),
        None if keys::is_known(key) => Ok(()),
        None => Err(KeyError::Unknown(key.to_owned()).into()),
    }
}

pub(super) fn toml(value: &str) -> Value {
    if let Ok(number) = value.parse::<i64>() {
        Value::from(number)
    } else if let Ok(number) = value.parse::<f64>() {
        Value::from(number)
    } else {
        Value::from(value)
    }
}

pub(super) fn read(path: &Path) -> Result<DocumentMut, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(ConfigError::Io {
                path: path.to_owned(),
                source,
            });
        }
    };
    text.parse().map_err(|source| ConfigError::Syntax {
        path: path.to_owned(),
        source,
    })
}

pub(super) fn text(item: &Item) -> Option<String> {
    match item.as_value()? {
        Value::String(text) => Some(text.value().clone()),
        Value::Integer(number) => Some(number.value().to_string()),
        Value::Float(number) => Some(number.value().to_string()),
        Value::Boolean(flag) => Some(flag.value().to_string()),
        Value::Datetime(datetime) => Some(datetime.value().to_string()),
        Value::Array(_) | Value::InlineTable(_) => None,
    }
}

fn write(path: &Path, document: &DocumentMut) -> Result<(), ConfigError> {
    let io_error = |source| ConfigError::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory).map_err(io_error)?;
    }
    fs::write(path, document.to_string()).map_err(io_error)
}

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use toml_edit::{DocumentMut, Item, TableLike, Value};

use super::keys::{self, KeyError, Scope, ScopedKey};
use super::{ConfigError, Entry};
use crate::provider::Provider;

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

pub(super) fn entries(document: &DocumentMut) -> Result<Vec<Entry>, KeyError> {
    let mut entries = Vec::new();
    for (name, item) in document.iter() {
        match item.as_table_like() {
            Some(section) => {
                let provider = name
                    .parse::<Provider>()
                    .map_err(|_| KeyError::Unknown(name.to_owned()))?;
                for (key, item) in section.iter() {
                    entries.push(entry(Scope::Only(provider), key, item)?);
                }
            }
            None => entries.push(entry(Scope::Every, name, item)?),
        }
    }
    Ok(entries)
}

pub(super) fn set(text: &str, value: &str) -> Result<(), ConfigError> {
    let key = ScopedKey::parse(text);
    keys::check(key, value)?;
    let path = path()?;
    let mut document = read(&path)?;
    let table: &mut dyn TableLike = match key.scope {
        Scope::Every => &mut *document,
        Scope::Only(provider) => document
            .entry(provider.id())
            .or_insert_with(toml_edit::table)
            .as_table_like_mut()
            .ok_or_else(|| KeyError::NotASection(provider.to_string()))?,
    };
    put(table, key.name, toml(value));
    write(&path, &document)
}

pub(super) fn unset(text: &str) -> Result<(), ConfigError> {
    let key = ScopedKey::parse(text);
    let path = path()?;
    let mut document = read(&path)?;
    let removed = match key.scope {
        Scope::Every => document.remove(key.name).is_some(),
        Scope::Only(provider) => remove_from_section(&mut document, provider, key.name),
    };
    if removed {
        write(&path, &document)
    } else if key.is_known() {
        Ok(())
    } else {
        Err(KeyError::Unknown(text.to_owned()).into())
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

fn text(item: &Item) -> Option<String> {
    match item.as_value()? {
        Value::String(text) => Some(text.value().clone()),
        Value::Integer(number) => Some(number.value().to_string()),
        Value::Float(number) => Some(number.value().to_string()),
        Value::Boolean(flag) => Some(flag.value().to_string()),
        Value::Datetime(datetime) => Some(datetime.value().to_string()),
        Value::Array(_) | Value::InlineTable(_) => None,
    }
}

fn entry(scope: Scope, name: &str, item: &Item) -> Result<Entry, KeyError> {
    let key = ScopedKey { scope, name };
    let value = text(item).ok_or_else(|| KeyError::NotSingle(key.to_string()))?;
    keys::check(key, &value)?;
    Ok(Entry {
        scope,
        name: name.to_owned(),
        value,
    })
}

fn put(table: &mut dyn TableLike, name: &str, mut new: Value) {
    match table.get_mut(name).and_then(Item::as_value_mut) {
        Some(old) => {
            *new.decor_mut() = old.decor().clone();
            *old = new;
        }
        None => {
            table.insert(name, Item::Value(new));
        }
    }
}

fn remove_from_section(document: &mut DocumentMut, provider: Provider, name: &str) -> bool {
    let Some(section) = document
        .get_mut(provider.id())
        .and_then(Item::as_table_like_mut)
    else {
        return false;
    };
    let removed = section.remove(name).is_some();
    if section.is_empty() {
        document.remove(provider.id());
    }
    removed
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

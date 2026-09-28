use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error};

use crate::provider::Provider;

const SERVICE: &str = "jevpipe";

#[derive(Debug, thiserror::Error)]
#[error("no keychain available here ({reason})")]
pub(crate) struct Unavailable {
    reason: String,
}

pub(super) enum Removed {
    Removed,
    NothingStored,
}

impl Unavailable {
    pub(crate) fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<Error> for Unavailable {
    fn from(error: Error) -> Self {
        Self {
            reason: error.to_string(),
        }
    }
}

pub(super) fn get(provider: Provider) -> Result<Option<String>, Unavailable> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn set(provider: Provider, key: &str) -> Result<(), Unavailable> {
    Ok(entry(provider)?.set_password(key)?)
}

pub(super) fn remove(provider: Provider) -> Result<Removed, Unavailable> {
    match entry(provider)?.delete_credential() {
        Ok(()) => Ok(Removed::Removed),
        Err(Error::NoEntry) => Ok(Removed::NothingStored),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn entry(provider: Provider) -> Result<Entry, Unavailable> {
    if keyring_core::get_default_store().is_none() {
        keyring_core::set_default_store(platform_store()?);
    }
    Ok(Entry::new(SERVICE, provider.keychain_user())?)
}

#[cfg(windows)]
fn platform_store() -> Result<Arc<CredentialStore>, Error> {
    Ok(windows_native_keyring_store::Store::new()?)
}

#[cfg(target_os = "linux")]
fn platform_store() -> Result<Arc<CredentialStore>, Error> {
    Ok(zbus_secret_service_keyring_store::Store::new()?)
}

#[cfg(target_os = "macos")]
fn platform_store() -> Result<Arc<CredentialStore>, Error> {
    Ok(super::security::Store::new()?)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn platform_store() -> Result<Arc<CredentialStore>, Error> {
    Err(Error::NotSupportedByStore(
        "jevpipe knows no keychain on this system".to_owned(),
    ))
}

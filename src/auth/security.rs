use std::any::Any;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

use keyring_core::api::{CredentialApi, CredentialStoreApi};
use keyring_core::{Credential, Entry, Error, Result};

const SECURITY: &str = "/usr/bin/security";
const NOT_FOUND: i32 = 44;

#[derive(Debug)]
pub(super) struct Store;

#[derive(Debug)]
struct Item {
    service: String,
    user: String,
}

impl Store {
    pub(super) fn new() -> Result<Arc<Self>> {
        if Path::new(SECURITY).exists() {
            Ok(Arc::new(Self))
        } else {
            Err(Error::NoStorageAccess(
                format!("{SECURITY} not found").into(),
            ))
        }
    }
}

impl CredentialStoreApi for Store {
    fn vendor(&self) -> String {
        SECURITY.to_owned()
    }

    fn id(&self) -> String {
        SECURITY.to_owned()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> Result<Entry> {
        Ok(Entry::new_with_credential(Arc::new(Item {
            service: service.to_owned(),
            user: user.to_owned(),
        })))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CredentialApi for Item {
    fn set_secret(&self, secret: &[u8]) -> Result<()> {
        let secret = str::from_utf8(secret)
            .map_err(|_| Error::Invalid("secret".to_owned(), "not UTF-8".to_owned()))?;
        let command = format!(
            "add-generic-password -U -s {} -a {} -w \"{secret}\"\n",
            self.service, self.user
        );
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(platform_failure)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(command.as_bytes())
                .map_err(platform_failure)?;
        }
        let output = child.wait_with_output().map_err(platform_failure)?;
        if output.stderr.is_empty() {
            checked(output).map(drop)
        } else {
            Err(failure(&output))
        }
    }

    fn get_secret(&self) -> Result<Vec<u8>> {
        let mut secret = self.run("find-generic-password", &["-w"])?;
        secret.pop_if(|byte| *byte == b'\n');
        Ok(secret)
    }

    fn delete_credential(&self) -> Result<()> {
        self.run("delete-generic-password", &[]).map(drop)
    }

    fn get_credential(&self) -> Result<Option<Arc<Credential>>> {
        self.get_secret().map(|_| None)
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        Some((self.service.clone(), self.user.clone()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Item {
    fn run(&self, command: &str, extra: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new(SECURITY)
            .args([command, "-s", &self.service, "-a", &self.user])
            .args(extra)
            .stdin(Stdio::null())
            .output()
            .map_err(platform_failure)?;
        checked(output)
    }
}

fn checked(output: Output) -> Result<Vec<u8>> {
    match output.status.code() {
        Some(0) => Ok(output.stdout),
        Some(NOT_FOUND) => Err(Error::NoEntry),
        Some(_) | None => Err(failure(&output)),
    }
}

fn failure(output: &Output) -> Error {
    let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Error::NoStorageAccess(format!("{SECURITY}: {message}").into())
}

fn platform_failure(error: std::io::Error) -> Error {
    Error::PlatformFailure(Box::new(error))
}

use std::process::ExitCode;

use super::keychain::{self, Removed};
use super::prompt::{self, Typed};
use crate::cli::AuthCommand;
use crate::config::{Config, ConfigError};
use crate::exit::{self, Exit};
use crate::output::report;
use crate::provider::Provider;

pub(crate) async fn run(command: AuthCommand, config: Result<Config, ConfigError>) -> ExitCode {
    let provider = match config {
        Ok(config) => config.provider(),
        Err(error) => return exit::fail(error),
    };
    match command {
        AuthCommand::SetKey => set_key(provider).await,
        AuthCommand::RemoveKey => remove_key(provider),
    }
}

async fn set_key(provider: Provider) -> ExitCode {
    match prompt::read_line(provider).await {
        Ok(Typed::Line(line)) => match super::store(provider, &line) {
            Ok(()) => {
                report(format_args!(
                    "{} API key stored in the keychain",
                    provider.name()
                ));
                Exit::Success.into()
            }
            Err(error) => exit::fail(error),
        },
        Ok(Typed::Interrupted) => {
            report("interrupted");
            Exit::Interrupted.into()
        }
        Err(error) => exit::fail(format_args!("cannot read the key: {error}")),
    }
}

fn remove_key(provider: Provider) -> ExitCode {
    match keychain::remove(provider) {
        Ok(Removed::Removed) => report(format_args!("{} API key removed", provider.name())),
        Ok(Removed::NothingStored) => {
            report(format_args!("no {} API key stored", provider.name()));
        }
        Err(error) => {
            return exit::fail(format_args!(
                "{error}; set {} instead",
                provider.key_variable()
            ));
        }
    }
    Exit::Success.into()
}

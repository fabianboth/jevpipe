use std::process::ExitCode;

use super::keychain::{self, Removed};
use super::prompt::{self, Typed};
use crate::cli::AuthCommand;
use crate::exit::{self, Exit};
use crate::output::report;

pub(crate) async fn run(command: AuthCommand) -> ExitCode {
    match command {
        AuthCommand::SetKey => set_key().await,
        AuthCommand::RemoveKey => remove_key(),
    }
}

async fn set_key() -> ExitCode {
    match prompt::read_line().await {
        Ok(Typed::Line(line)) => match super::store(&line) {
            Ok(()) => {
                report("API key stored in the keychain");
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

fn remove_key() -> ExitCode {
    match keychain::remove() {
        Ok(Removed::Removed) => report("API key removed"),
        Ok(Removed::NothingStored) => report("no API key stored"),
        Err(error) => return exit::fail(error),
    }
    Exit::Success.into()
}

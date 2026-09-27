use std::io::Write;
use std::process::ExitCode;

use super::{Config, ConfigError, KeyError, file, keys};
use crate::auth;
use crate::cli::ConfigCommand;
use crate::exit::{self, Exit};
use crate::output::Output;

pub(crate) fn run(command: ConfigCommand, config: Result<Config, ConfigError>) -> ExitCode {
    let text = match command {
        ConfigCommand::List => config.and_then(|config| Ok(list(&config)?)),
        ConfigCommand::Get { key } => {
            config.and_then(|config| Ok(format!("{}\n", config.setting(&key)?.value)))
        }
        ConfigCommand::Set { key, value } => file::set(&key, &value).map(|()| String::new()),
        ConfigCommand::Unset { key } => file::unset(&key).map(|()| String::new()),
        ConfigCommand::Path => file::path().map(|path| format!("{}\n", path.display())),
    };
    match text.and_then(|text| print(&text)) {
        Ok(()) => Exit::Success.into(),
        Err(error) => exit::fail(error),
    }
}

fn list(config: &Config) -> Result<String, KeyError> {
    let settings = keys::all()
        .into_iter()
        .map(|key| {
            let setting = config.setting(&key)?;
            Ok((
                format!("{key} = {}", file::toml(&setting.value)),
                setting.origin,
            ))
        })
        .collect::<Result<Vec<_>, KeyError>>()?;
    let width = settings
        .iter()
        .map(|(line, _)| line.len())
        .max()
        .unwrap_or_default();
    let mut lines: Vec<_> = settings
        .iter()
        .map(|(line, origin)| format!("{line:width$}  # {origin}\n"))
        .collect();
    lines.push(format!("# API key: {}\n", auth::source()));
    Ok(lines.concat())
}

fn print(text: &str) -> Result<(), ConfigError> {
    Output::new()
        .write(|out| out.write_all(text.as_bytes()))
        .map(drop)
        .map_err(ConfigError::Output)
}

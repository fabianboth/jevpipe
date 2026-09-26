mod cli;
mod decision;
mod file;
mod filter;
mod output;
mod record;
mod service;
mod summary;
mod text;

use std::process::ExitCode;

pub use cli::Cli;

pub async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        cli::Command::Filter(args) => filter::run(args).await,
    }
}

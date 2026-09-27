mod answers;
mod auth;
mod cli;
mod config;
mod cost;
mod decision;
mod exit;
mod file;
mod filter;
mod limits;
mod map;
mod output;
mod pipeline;
mod questions;
mod reason;
mod record;
mod service;
mod settings;
mod summary;
mod text;

use std::process::ExitCode;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::exit::Exit;
use crate::filter::Filter;
use crate::map::Map;

pub async fn run() -> ExitCode {
    let config = Config::load();
    let cli = Cli::parse_with(config.as_ref().ok());
    match cli.command {
        Commands::Filter(args) => {
            let filter = Filter::new(args.question, args.threshold);
            run_pipeline(filter, args.run, config).await
        }
        Commands::Map(args) => match args.questions.into_questions() {
            Ok(questions) => run_pipeline(Map::new(questions), args.run, config).await,
            Err(error) => {
                let _ = error.print();
                Exit::Error.into()
            }
        },
        Commands::Config(command) => config::run(command, config),
        Commands::Auth(command) => auth::run(command).await,
    }
}

async fn run_pipeline(
    command: impl pipeline::Command,
    args: cli::RunArgs,
    config: Result<Config, config::ConfigError>,
) -> ExitCode {
    match config {
        Ok(config) => pipeline::run(command, args, &config).await,
        Err(error) => exit::fail(error),
    }
}

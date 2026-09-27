mod answers;
mod cli;
mod decision;
mod file;
mod filter;
mod map;
mod output;
mod pipeline;
mod questions;
mod reason;
mod record;
mod service;
mod summary;
mod text;

use std::process::ExitCode;

pub use cli::Cli;

use crate::cli::Commands;
use crate::filter::Filter;
use crate::map::Map;
use crate::summary::Exit;

pub async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Commands::Filter(args) => {
            let filter = Filter::new(args.question, args.threshold);
            pipeline::run(filter, args.files, args.run).await
        }
        Commands::Map(args) => match args.questions.into_questions() {
            Ok(questions) => pipeline::run(Map::new(questions), args.files, args.run).await,
            Err(error) => {
                let _ = error.print();
                Exit::Error.into()
            }
        },
    }
}

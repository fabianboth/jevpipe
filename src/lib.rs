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

use crate::filter::Filter;
use crate::map::Map;

pub async fn run(cli: Cli) -> ExitCode {
    match cli.command {
        cli::Command::Filter(args) => {
            let filter = Filter::new(args.question, args.threshold);
            pipeline::run(filter, args.files, args.run).await
        }
        cli::Command::Map(args) => {
            pipeline::run(Map::new(args.questions), args.files, args.run).await
        }
    }
}

use std::process::ExitCode;

use clap::Parser;
use jevpipe::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    jevpipe::run(Cli::parse()).await
}

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    jevpipe::run().await
}

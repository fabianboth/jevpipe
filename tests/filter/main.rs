#![expect(
    clippy::unwrap_used,
    reason = "integration tests: a failed setup should fail the test"
)]

mod files;
mod fixture;
mod json;
mod lines;
mod stand_in;
mod streaming;

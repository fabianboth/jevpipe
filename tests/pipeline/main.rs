#![expect(
    clippy::unwrap_used,
    reason = "integration tests: a failed setup should fail the test"
)]

mod filter_files;
mod filter_lines;
mod fixture;
mod map_answers;
mod map_files;
mod stand_in;
mod stderr;
mod streaming;

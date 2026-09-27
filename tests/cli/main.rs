#![expect(
    clippy::unwrap_used,
    reason = "integration tests: a failed setup should fail the test"
)]

mod auth;
mod config;
mod help;
mod home;
mod usage;

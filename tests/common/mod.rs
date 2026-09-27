use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;

const NO_CONFIG: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/no-config/config.toml");

pub(crate) fn jevpipe() -> Command {
    let mut command = cargo_bin_cmd!();
    command.env("JEVPIPE_CONFIG", NO_CONFIG);
    command
}

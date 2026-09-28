use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use assert_cmd::Command;
use tempfile::TempDir;

pub(crate) const API_KEY: &str = "sk-or-v1-test-9f8e7d";

const NO_CONFIG: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/no-config/config.toml");

pub(crate) struct Home {
    dir: TempDir,
}

pub(crate) fn jevpipe() -> Command {
    Command::from_std(binary(Path::new(NO_CONFIG)))
}

pub(crate) fn stdout(command: &mut Command) -> String {
    let assert = command.assert().success();
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

impl Home {
    pub(crate) fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.dir.path().join("jevpipe").join("config.toml")
    }

    pub(crate) fn write(&self, text: &str) {
        fs::create_dir_all(self.dir.path().join("jevpipe")).unwrap();
        fs::write(self.config(), text).unwrap();
    }

    pub(crate) fn read(&self) -> String {
        fs::read_to_string(self.config()).unwrap()
    }

    pub(crate) fn command(&self) -> process::Command {
        binary(&self.config())
    }

    pub(crate) fn jevpipe(&self) -> Command {
        Command::from_std(self.command())
    }

    pub(crate) fn config_command(&self, args: &[&str]) -> Command {
        let mut command = self.jevpipe();
        command.arg("config").args(args);
        command
    }

    pub(crate) fn stdout(&self, args: &[&str]) -> String {
        stdout(&mut self.config_command(args))
    }
}

fn binary(config: &Path) -> process::Command {
    let mut command = process::Command::new(env!("CARGO_BIN_EXE_jevpipe"));
    command
        .env("JEVPIPE_CONFIG", config)
        .env_remove("TYPESAFE_API_KEY")
        .env("OPENROUTER_API_KEY", API_KEY);
    command
}

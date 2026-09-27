#![expect(
    clippy::unwrap_used,
    reason = "integration tests: a failed setup should fail the test"
)]

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::TempDir;
use toml_edit::DocumentMut;

struct Home {
    dir: TempDir,
}

impl Home {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    fn config(&self) -> PathBuf {
        self.dir.path().join("jevpipe").join("config.toml")
    }

    fn write(&self, text: &str) {
        fs::create_dir_all(self.dir.path().join("jevpipe")).unwrap();
        fs::write(self.config(), text).unwrap();
    }

    fn read(&self) -> String {
        fs::read_to_string(self.config()).unwrap()
    }

    fn jevpipe(&self) -> Command {
        let mut command = cargo_bin_cmd!();
        command
            .env("JEVPIPE_CONFIG", self.config())
            .env("OPENROUTER_API_KEY", "test-key");
        command
    }

    fn config_command(&self, args: &[&str]) -> Command {
        let mut command = self.jevpipe();
        command.arg("config").args(args);
        command
    }

    fn stdout(&self, args: &[&str]) -> String {
        let assert = self.config_command(args).assert().success();
        String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
    }
}

#[test]
fn set_creates_the_file_and_its_directory() {
    let home = Home::new();

    home.config_command(&["set", "model", "typesafe/jev-1.13"])
        .assert()
        .success()
        .stdout("");

    assert_eq!(home.read(), "model = \"typesafe/jev-1.13\"\n");
}

#[test]
fn set_and_unset_keep_the_other_keys_and_comments() {
    let home = Home::new();
    home.write(
        "# my defaults\nmodel = \"x\"   # pinned\nmax-cost = 1.0\n\n# waiting\nmax-time = \"1h\"\n",
    );

    home.config_command(&["set", "model", "typesafe/jev-1.13"])
        .assert()
        .success();
    home.config_command(&["set", "concurrency", "4"])
        .assert()
        .success();
    home.config_command(&["unset", "max-cost"])
        .assert()
        .success();

    assert_eq!(
        home.read(),
        "# my defaults\nmodel = \"typesafe/jev-1.13\"   # pinned\n\n# waiting\nmax-time = \"1h\"\nconcurrency = 4\n"
    );
}

#[test]
fn set_checks_the_value_like_the_flag_and_leaves_the_file_alone() {
    let home = Home::new();
    home.write("model = \"x\"\n");
    let cases = [
        (
            ["concurrency", "0"],
            "invalid value '0' for `concurrency`: number would be zero for non-zero type",
        ),
        (
            ["max-time", "10"],
            "invalid value '10' for `max-time`: needs a unit, for example 10s or 5m",
        ),
        (
            ["max-cost", "free"],
            "invalid value 'free' for `max-cost`: not an amount of US dollars, for example 0.5 or 2",
        ),
        (
            ["base-url", "ftp://example.com"],
            "invalid value 'ftp://example.com' for `base-url`: not an http or https URL",
        ),
        (
            ["concurency", "4"],
            "unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, model, request-timeout",
        ),
    ];
    for (args, problem) in cases {
        home.config_command(&["set", args[0], args[1]])
            .assert()
            .code(2)
            .stderr(format!("jevpipe: error: {problem}\n"));
    }

    assert_eq!(home.read(), "model = \"x\"\n");
}

#[test]
fn reading_commands_name_a_broken_file() {
    let home = Home::new();
    home.write("concurency = 4\n");
    let problem = format!(
        "jevpipe: error: {}: unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, model, request-timeout\n",
        home.config().display()
    );

    for args in [&["list"][..], &["get", "model"]] {
        home.config_command(args)
            .assert()
            .code(2)
            .stdout("")
            .stderr(problem.clone());
    }
}

#[test]
fn a_broken_file_can_still_be_found_and_repaired() {
    let home = Home::new();
    home.write("concurency = 4\nmodel = \"\"\n");

    assert_eq!(
        home.stdout(&["path"]),
        format!("{}\n", home.config().display())
    );
    home.config_command(&["unset", "concurency"])
        .assert()
        .success();
    home.config_command(&["set", "model", "typesafe/jev-1.13"])
        .assert()
        .success();

    assert_eq!(home.read(), "model = \"typesafe/jev-1.13\"\n");
    home.config_command(&["list"]).assert().success();
}

#[test]
fn a_file_that_is_not_toml_is_named_but_its_path_still_shown() {
    let home = Home::new();
    home.write("model = \n");

    home.config_command(&["set", "concurrency", "4"])
        .assert()
        .code(2)
        .stderr(contains(format!(
            "jevpipe: error: {}: TOML parse error",
            home.config().display()
        )));
    home.config_command(&["path"]).assert().success();
}

#[test]
fn list_prints_every_setting_as_toml_with_its_origin() {
    let home = Home::new();
    home.write("max-cost = 0.5\nrequest-timeout = \"20s\"\n");

    let list = home.stdout(&["list"]);

    let document: DocumentMut = list.parse().unwrap();
    let keys: Vec<_> = document.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(
        keys,
        [
            "base-url",
            "concurrency",
            "max-cost",
            "max-time",
            "model",
            "request-timeout"
        ]
    );
    assert_eq!(document["concurrency"].as_integer(), Some(100));
    assert_eq!(document["max-cost"].as_float(), Some(0.5));
    assert_eq!(document["max-time"].as_str(), Some("none"));
    assert_eq!(document["request-timeout"].as_str(), Some("20s"));
    assert_eq!(
        document["base-url"].as_str(),
        Some("https://openrouter.ai/api")
    );
    for (key, origin) in [
        ("concurrency", "default"),
        ("max-cost", "config file"),
        ("request-timeout", "config file"),
    ] {
        let line = list.lines().find(|line| line.starts_with(key)).unwrap();
        assert!(line.ends_with(&format!("# {origin}")), "{line}");
    }
    assert!(
        list.ends_with("# API key: from OPENROUTER_API_KEY\n"),
        "{list}"
    );
    assert!(!list.contains("test-key"), "{list}");
}

#[test]
fn get_prints_only_the_effective_value() {
    let home = Home::new();
    home.write("request-timeout = \"20s\"\n");

    for (key, value) in [
        ("concurrency", "100"),
        ("max-cost", "none"),
        ("request-timeout", "20s"),
    ] {
        assert_eq!(home.stdout(&["get", key]), format!("{value}\n"));
    }
    home.config_command(&["get", "concurency"])
        .assert()
        .code(2)
        .stderr(contains("unknown key `concurency`"));
}

#[test]
fn path_prints_the_file_in_use_whether_or_not_it_exists() {
    let home = Home::new();

    assert_eq!(
        home.stdout(&["path"]),
        format!("{}\n", home.config().display())
    );
    assert!(!home.config().exists());
}

#[test]
fn help_shows_the_configured_defaults_and_falls_back_on_a_broken_file() {
    let home = Home::new();
    home.write("concurrency = 4\nmax-time = \"10m\"\n");
    let help = |home: &Home| {
        let assert = home.jevpipe().args(["map", "--help"]).assert().success();
        String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
    };

    let configured = help(&home);
    assert!(configured.contains("[default: 4]"), "{configured}");
    assert!(configured.contains("[default: 10m]"), "{configured}");

    home.write("concurrency = 0\n");
    let fallback = help(&home);
    assert!(fallback.contains("[default: 100]"), "{fallback}");
}

#[test]
fn config_help_names_the_keys_the_file_and_the_order() {
    let assert = Home::new()
        .jevpipe()
        .args(["config", "--help"])
        .assert()
        .success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for part in [
        "list",
        "get",
        "set",
        "unset",
        "path",
        "JEVPIPE_CONFIG",
        "base-url",
        "max-cost",
    ] {
        assert!(help.contains(part), "missing {part} in:\n{help}");
    }
}

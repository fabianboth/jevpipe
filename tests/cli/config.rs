use std::io;
use std::process::Stdio;

use predicates::str::contains;
use toml_edit::DocumentMut;

use crate::home::{API_KEY, Home, stdout};

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
            "unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, max-tokens, model, provider, request-timeout, and all but provider also as openrouter.<key> or typesafe.<key>",
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
        "jevpipe: error: {}: unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, max-tokens, model, provider, request-timeout, and all but provider also as openrouter.<key> or typesafe.<key>\n",
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
            "max-tokens",
            "model",
            "provider",
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
    assert!(!list.contains(API_KEY), "{list}");
}

#[test]
fn the_provider_decides_the_address_and_whose_key_is_shown() {
    let home = Home::new();
    home.config_command(&["set", "provider", "typesafe"])
        .assert()
        .success();
    home.config_command(&["set", "provider", "anthropic"])
        .assert()
        .code(2)
        .stderr(
            "jevpipe: error: invalid value 'anthropic' for `provider`: the providers are openrouter, typesafe\n",
        );
    assert_eq!(home.read(), "provider = \"typesafe\"\n");
    assert_eq!(
        home.stdout(&["get", "base-url"]),
        "https://api.typesafe.ai\n"
    );

    let list = stdout(
        home.config_command(&["list"])
            .env("TYPESAFE_API_KEY", "ts-test-key"),
    );

    for (start, origin) in [
        ("provider = \"typesafe\"", "config file"),
        ("base-url = \"https://api.typesafe.ai\"", "default"),
        ("model = \"jev-latest\"", "default"),
    ] {
        let line = list.lines().find(|line| line.starts_with(start)).unwrap();
        assert!(line.ends_with(&format!("# {origin}")), "{line}");
    }
    assert!(
        list.ends_with("# API key: from TYPESAFE_API_KEY\n"),
        "{list}"
    );
    assert!(!list.contains("ts-test-key"), "{list}");
}

#[test]
fn the_token_limit_takes_counts_with_a_suffix() {
    let home = Home::new();
    for value in ["250k", "1.5M", "1000000", "none", "5M"] {
        home.config_command(&["set", "max-tokens", value])
            .assert()
            .success();
        assert_eq!(home.stdout(&["get", "max-tokens"]), format!("{value}\n"));
    }
    home.config_command(&["set", "max-tokens", "0"])
        .assert()
        .code(2)
        .stderr(contains("must be positive"));
    assert_eq!(home.read(), "max-tokens = \"5M\"\n");
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
fn reading_commands_end_quietly_when_the_reader_is_gone() {
    let home = Home::new();
    for args in [&["list"][..], &["get", "model"], &["path"]] {
        let (reader, writer) = io::pipe().unwrap();
        drop(reader);

        let output = home
            .command()
            .arg("config")
            .args(args)
            .stdout(writer)
            .stderr(Stdio::piped())
            .output()
            .unwrap();

        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
    }
}

#[test]
fn a_provider_prefix_edits_that_providers_section() {
    let home = Home::new();
    home.write("# mine\nconcurrency = 50\n");

    for (key, value) in [
        ("openrouter.max-cost", "0.5"),
        ("openrouter.model", "jev-1.13"),
        ("typesafe.model", "jev-1.13.0"),
    ] {
        home.config_command(&["set", key, value]).assert().success();
    }
    assert_eq!(
        home.read(),
        "# mine\nconcurrency = 50\n\n[openrouter]\nmax-cost = 0.5\nmodel = \"jev-1.13\"\n\n[typesafe]\nmodel = \"jev-1.13.0\"\n"
    );

    home.config_command(&["unset", "typesafe.model"])
        .assert()
        .success();
    home.config_command(&["unset", "openrouter.model"])
        .assert()
        .success();
    assert_eq!(
        home.read(),
        "# mine\nconcurrency = 50\n\n[openrouter]\nmax-cost = 0.5\n"
    );
}

#[test]
fn the_active_providers_section_beats_the_top_level() {
    let home = Home::new();
    home.write(
        "provider = \"typesafe\"\nmodel = \"jev-latest\"\nconcurrency = 50\n\n[openrouter]\nmodel = \"jev-1.13\"\nmax-cost = 0.5\n\n[typesafe]\nmodel = \"jev-1.13.0\"\n",
    );

    assert_eq!(home.stdout(&["get", "model"]), "jev-1.13.0\n");
    assert_eq!(home.stdout(&["get", "openrouter.model"]), "jev-1.13\n");
    assert_eq!(home.stdout(&["get", "openrouter.max-cost"]), "0.5\n");
    assert_eq!(home.stdout(&["get", "max-cost"]), "none\n");
    assert_eq!(home.stdout(&["get", "openrouter.concurrency"]), "50\n");

    let list = stdout(
        home.config_command(&["list"])
            .env("TYPESAFE_API_KEY", "ts-test-key"),
    );
    for (start, origin) in [
        ("model = \"jev-1.13.0\"", "config file [typesafe]"),
        ("concurrency = 50", "config file"),
        ("max-cost = \"none\"", "default"),
    ] {
        let line = list.lines().find(|line| line.starts_with(start)).unwrap();
        assert!(line.ends_with(&format!("# {origin}")), "{line}");
    }
}

#[test]
fn keys_that_do_not_fit_a_section_are_refused() {
    let home = Home::new();
    home.write("model = \"x\"\n");
    let cases = [
        (
            ["typesafe.max-cost", "1"],
            "invalid value '1' for `typesafe.max-cost`: TypeSafe reports no cost; use typesafe.max-tokens",
        ),
        (
            ["openrouter.provider", "typesafe"],
            "unknown key `openrouter.provider`",
        ),
        (["anthropic.model", "x"], "unknown key `anthropic.model`"),
        (
            ["openrouter.concurrency", "0"],
            "invalid value '0' for `openrouter.concurrency`",
        ),
    ];
    for (args, problem) in cases {
        home.config_command(&["set", args[0], args[1]])
            .assert()
            .code(2)
            .stderr(contains(format!("jevpipe: error: {problem}")));
    }
    assert_eq!(home.read(), "model = \"x\"\n");

    for (text, problem) in [
        ("[anthropic]\nmodel = \"x\"\n", "unknown key `anthropic`"),
        (
            "[openrouter]\nprovider = \"typesafe\"\n",
            "unknown key `openrouter.provider`",
        ),
        (
            "[typesafe]\nmax-cost = 1\n",
            "invalid value '1' for `typesafe.max-cost`: TypeSafe reports no cost",
        ),
    ] {
        home.write(text);
        home.config_command(&["list"])
            .assert()
            .code(2)
            .stderr(contains(format!(
                "jevpipe: error: {}: {problem}",
                home.config().display()
            )));
    }
}

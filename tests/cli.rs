mod common;

use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

use common::jevpipe;

#[test]
fn version_reports_the_crate_version() {
    jevpipe()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn filter_rejects_invalid_usage_before_reading_input() {
    let cases: [&[&str]; 7] = [
        &["--no-such-flag"],
        &["filter", "Is it?", "--json"],
        &["filter", "Is it?", "--all"],
        &["filter", ""],
        &["filter", "Is it?", "--threshold", "1.5"],
        &["filter", "Is it?", "--concurrency", "0"],
        &["filter", "Is it?", "--request-timeout", "0"],
    ];
    for args in cases {
        jevpipe()
            .args(args)
            .env("OPENROUTER_API_KEY", "test-key")
            .assert()
            .code(2)
            .stderr(contains("error:"));
    }
}

#[test]
fn durations_need_a_unit_and_must_be_longer_than_zero() {
    let cases = [
        ("10", "needs a unit, for example 10s or 5m"),
        ("0s", "must be longer than zero"),
    ];
    for (value, problem) in cases {
        jevpipe()
            .args(["filter", "Is it?", "--request-timeout", value])
            .env("OPENROUTER_API_KEY", "test-key")
            .write_stdin("a\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains(format!(
                "invalid value '{value}' for '--request-timeout <DURATION>': {problem}"
            )));
    }
}

#[test]
fn limits_must_be_positive_or_none() {
    let cases = [
        ("--max-cost", "0", "must be more than zero"),
        ("--max-cost", "-1", "not an amount of US dollars"),
        ("--max-cost", "abc", "not an amount of US dollars"),
        ("--max-time", "10", "needs a unit, for example 10s or 5m"),
        ("--max-time", "0s", "must be longer than zero"),
    ];
    for (option, value, problem) in cases {
        jevpipe()
            .args([
                "map",
                "-q",
                r#"{"q": {"type": "noul", "instructions": "Is it?"}}"#,
            ])
            .arg(format!("{option}={value}"))
            .env("OPENROUTER_API_KEY", "test-key")
            .write_stdin("a\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(
                contains(format!("invalid value '{value}' for '{option}")).and(contains(problem)),
            );
    }
}

#[test]
fn run_help_describes_the_limits_and_their_exit_status() {
    for command in ["filter", "map"] {
        let assert = jevpipe().args([command, "--help"]).assert().success();
        let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
        for part in [
            "--max-cost <DOLLARS|none>",
            "--max-time <DURATION|none>",
            "--request-timeout <DURATION>",
            "3  a limit stopped the run early; standard error names the line to resume from",
        ] {
            assert!(help.contains(part), "missing {part} in:\n{help}");
        }
    }
}

#[test]
fn filter_help_describes_every_option() {
    let assert = jevpipe().args(["filter", "--help"]).assert().success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for option in [
        "Usage: jevpipe filter [OPTIONS] <QUESTION> [FILES]...",
        "<QUESTION>",
        "[FILES]...",
        "--read-files",
        "--threshold",
        "--concurrency",
        "--model",
        "--request-timeout",
        "Examples:",
    ] {
        assert!(help.contains(option), "missing {option} in:\n{help}");
    }
}

fn options(count: usize) -> String {
    let options: Vec<_> = (0..count)
        .map(|option| format!("\"o{option}\": \"\""))
        .collect();
    format!(
        r#"{{"q": {{"type": "choice", "instructions": "Which?", "criteria": {{{}}}}}}}"#,
        options.join(", ")
    )
}

fn levels(count: usize) -> String {
    let levels = vec!["\"level\""; count].join(", ");
    format!(r#"{{"q": {{"type": "score", "instructions": "How much?", "criteria": [{levels}]}}}}"#)
}

#[test]
fn map_rejects_a_malformed_questions_file_before_reading_input() {
    let (no_options, too_many_options) = (options(0), options(256));
    let (one_level, too_many_levels) = (levels(1), levels(11));
    let cases = [
        ("{", "not JSON"),
        ("[]", "must be an object of named questions"),
        ("{}", "has no questions"),
        (
            r#"{"q": {"type": "noul", "instructions": "Is it?"}, "q": {"type": "noul", "instructions": "Is it?"}}"#,
            "question `q` appears twice",
        ),
        (
            r#"{"q": "Is it?"}"#,
            "question `q`: invalid type: string \"Is it?\", expected an object with type and instructions",
        ),
        (
            r#"{"q": {"type": "maybe", "instructions": "Is it?"}}"#,
            "question `q`: unknown variant `maybe`",
        ),
        (
            r#"{"q": {"type": "noul"}}"#,
            "question `q`: needs instructions",
        ),
        (
            r#"{"q": {"type": "noul", "instructions": null}}"#,
            "question `q`: needs instructions",
        ),
        (
            r#"{"q": {"type": "noul", "instructions": "Is it?", "criteria": "yes"}}"#,
            "question `q`: a noul's criteria must be an object",
        ),
        (
            r#"{"q": {"type": "choice", "instructions": "Which?"}}"#,
            "question `q`: a choice needs criteria with 1 to 255 options",
        ),
        (
            no_options.as_str(),
            "question `q`: a choice needs criteria with 1 to 255 options",
        ),
        (
            too_many_options.as_str(),
            "question `q`: a choice needs criteria with 1 to 255 options",
        ),
        (
            one_level.as_str(),
            "question `q`: a score needs criteria with 2 to 10 levels",
        ),
        (
            too_many_levels.as_str(),
            "question `q`: a score needs criteria with 2 to 10 levels",
        ),
        (
            r#"{"q": {"type": "score", "instructions": "How much?", "criteria": {"low": ""}}}"#,
            "question `q`: a score needs criteria with 2 to 10 levels",
        ),
    ];
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("questions.json");
    for (content, problem) in cases {
        std::fs::write(&file, content).unwrap();
        jevpipe()
            .current_dir(dir.path())
            .args(["map", "-f", "questions.json"])
            .env("OPENROUTER_API_KEY", "test-key")
            .write_stdin("a\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains("questions.json").and(contains(problem)));
    }
}

#[test]
fn map_accepts_the_largest_questions_the_service_allows() {
    let dir = tempfile::TempDir::new().unwrap();
    for content in [options(1), options(255), levels(2), levels(10)] {
        std::fs::write(dir.path().join("questions.json"), content).unwrap();
        jevpipe()
            .current_dir(dir.path())
            .args(["map", "-f", "questions.json"])
            .env("OPENROUTER_API_KEY", "test-key")
            .write_stdin("")
            .assert()
            .code(0);
    }
}

#[test]
fn map_names_a_missing_questions_file() {
    jevpipe()
        .args(["map", "-f", "no-such-questions.json"])
        .env("OPENROUTER_API_KEY", "test-key")
        .write_stdin("")
        .assert()
        .code(2)
        .stderr(contains("no-such-questions.json").and(contains("not found")));
}

#[test]
fn map_needs_exactly_one_source_of_questions() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("questions.json"),
        r#"{"q": {"type": "noul", "instructions": "Is it?"}}"#,
    )
    .unwrap();
    let inline = r#"{"q": {"type": "noul", "instructions": "Is it?"}}"#;
    let cases: [(&[&str], &str); 2] = [
        (&["map"], "required arguments were not provided"),
        (
            &["map", "-q", inline, "-f", "questions.json"],
            "cannot be used with",
        ),
    ];
    for (args, problem) in cases {
        jevpipe()
            .current_dir(dir.path())
            .args(args)
            .env("OPENROUTER_API_KEY", "test-key")
            .write_stdin("")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains(problem));
    }
}

#[test]
fn map_checks_inline_questions_like_a_questions_file() {
    jevpipe()
        .args([
            "map",
            "-q",
            r#"{"q": {"type": "score", "instructions": "How much?", "criteria": ["only"]}}"#,
        ])
        .env("OPENROUTER_API_KEY", "test-key")
        .write_stdin("")
        .assert()
        .code(2)
        .stderr(contains("--questions").and(contains(
            "question `q`: a score needs criteria with 2 to 10 levels",
        )));
}

#[test]
fn map_help_describes_every_option_and_the_questions_file() {
    let assert = jevpipe().args(["map", "--help"]).assert().success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for part in [
        "Usage: jevpipe map [OPTIONS] <--questions <JSON>|--questions-file <FILE>> [FILES]...",
        "-q, --questions <JSON>",
        "-f, --questions-file <FILE>",
        "[FILES]...",
        "--read-files",
        "--concurrency",
        "--model",
        "--request-timeout",
        "\"type\": \"noul\"",
        "\"type\": \"choice\"",
        "\"type\": \"score\"",
        "jq",
    ] {
        assert!(help.contains(part), "missing {part} in:\n{help}");
    }
    assert!(!help.contains("--threshold"), "{help}");
}

#[test]
fn map_short_help_keeps_the_examples_and_leaves_out_the_formats() {
    let assert = jevpipe().args(["map", "-h"]).assert().success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(help.contains("Examples:"), "{help}");
    assert!(help.contains("-q, --questions <JSON>"), "{help}");
    assert!(!help.contains("\"type\": \"score\""), "{help}");
}

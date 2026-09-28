use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

use crate::home::jevpipe;

#[test]
fn version_reports_the_crate_version() {
    jevpipe()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_options_and_an_empty_question_are_usage_errors() {
    let cases: [&[&str]; 4] = [
        &["--no-such-flag"],
        &["filter", "Is it?", "--json"],
        &["filter", "Is it?", "--all"],
        &["filter", ""],
    ];
    for args in cases {
        jevpipe()
            .args(args)
            .write_stdin("a\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains("error:"));
    }
}

#[test]
fn option_values_are_checked_before_any_input_is_read() {
    let cases = [
        ("--threshold", "1.5", "`1.5` is not between 0 and 1"),
        (
            "--concurrency",
            "0",
            "number would be zero for non-zero type",
        ),
        (
            "--request-timeout",
            "10",
            "needs a unit, for example 10s or 5m",
        ),
        ("--request-timeout", "0s", "must be longer than zero"),
        ("--max-cost", "0", "must be more than zero"),
        ("--max-cost", "-1", "not an amount of US dollars"),
        ("--max-cost", "abc", "not an amount of US dollars"),
        ("--max-time", "10", "needs a unit, for example 10s or 5m"),
        ("--max-time", "0s", "must be longer than zero"),
        ("--max-tokens", "0", "must be positive"),
        ("--max-tokens", "-1", "not a count, for example 250k or 5M"),
        (
            "--max-tokens",
            "lots",
            "not a count, for example 250k or 5M",
        ),
    ];
    for (option, value, problem) in cases {
        jevpipe()
            .args(["filter", "Is it?"])
            .arg(format!("{option}={value}"))
            .write_stdin("a\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(
                contains(format!("invalid value '{value}' for '{option}")).and(contains(problem)),
            );
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
            .write_stdin("")
            .assert()
            .code(0);
    }
}

#[test]
fn map_names_a_missing_questions_file() {
    jevpipe()
        .args(["map", "-f", "no-such-questions.json"])
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
        .write_stdin("")
        .assert()
        .code(2)
        .stderr(contains("--questions").and(contains(
            "question `q`: a score needs criteria with 2 to 10 levels",
        )));
}

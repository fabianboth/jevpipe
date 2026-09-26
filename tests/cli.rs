use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

#[test]
fn version_reports_the_crate_version() {
    cargo_bin_cmd!()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn filter_rejects_invalid_usage_before_reading_input() {
    let cases: [&[&str]; 6] = [
        &["--no-such-flag"],
        &["filter", "Is it?", "--all"],
        &["filter", ""],
        &["filter", "Is it?", "--threshold", "1.5"],
        &["filter", "Is it?", "--concurrency", "0"],
        &["filter", "Is it?", "--request-timeout", "0"],
    ];
    for args in cases {
        cargo_bin_cmd!()
            .args(args)
            .env("OPENROUTER_API_KEY", "test-key")
            .assert()
            .code(2)
            .stderr(contains("error:"));
    }
}

#[test]
fn filter_help_describes_every_option() {
    let assert = cargo_bin_cmd!()
        .args(["filter", "--help"])
        .assert()
        .success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for option in [
        "Usage: jevpipe filter [OPTIONS] <QUESTION> [FILES]...",
        "<QUESTION>",
        "[FILES]...",
        "--read-files",
        "--threshold",
        "--json",
        "--all",
        "--concurrency",
        "--model",
        "--request-timeout",
        "OPENROUTER_API_KEY",
    ] {
        assert!(help.contains(option), "missing {option} in:\n{help}");
    }
}

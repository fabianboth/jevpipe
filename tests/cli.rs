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
fn unknown_flag_fails_with_usage_error() {
    cargo_bin_cmd!()
        .arg("--no-such-flag")
        .assert()
        .code(2)
        .stderr(contains("--no-such-flag"));
}

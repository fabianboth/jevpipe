mod common;

use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

use common::jevpipe;

#[test]
fn an_empty_key_is_refused_before_the_keychain_is_opened() {
    for input in ["", "\n", "   \n"] {
        jevpipe()
            .args(["auth", "set-key"])
            .write_stdin(input)
            .assert()
            .code(2)
            .stdout("")
            .stderr("jevpipe: error: no API key given\n");
    }
}

#[test]
fn a_key_with_characters_no_api_key_has_is_refused() {
    for key in [
        "sk-or v1",
        "sk-or-\"v1",
        "sk-or-'v1",
        "sk-or-\\v1",
        "sk-or-v1-é",
        "sk-or-v1-\u{200B}abc",
        "sk-or-\tv1",
    ] {
        jevpipe()
            .args(["auth", "set-key"])
            .write_stdin(format!("{key}\n"))
            .assert()
            .code(2)
            .stderr(
                "jevpipe: error: that does not look like an API key (printable ASCII characters only, no spaces, quotes or backslashes)\n",
            );
    }
}

#[test]
fn the_key_is_never_taken_as_an_argument() {
    jevpipe()
        .args(["auth", "set-key", "sk-or-v1-abc"])
        .write_stdin("")
        .assert()
        .code(2)
        .stderr(contains("unexpected argument 'sk-or-v1-abc'"));
}

#[test]
fn a_piped_key_gets_no_prompt() {
    let assert = jevpipe()
        .args(["auth", "set-key"])
        .write_stdin("sk-or bad\n")
        .assert()
        .code(2);

    assert!(!String::from_utf8_lossy(&assert.get_output().stderr).contains("OpenRouter API key"));
}

#[test]
fn config_list_names_the_variable_and_never_shows_the_key() {
    let assert = jevpipe()
        .args(["config", "list"])
        .env("OPENROUTER_API_KEY", "sk-or-v1-secret-9f8e7d")
        .assert()
        .success()
        .stdout(contains("# API key: from OPENROUTER_API_KEY\n"));

    let output = assert.get_output();
    for stream in [&output.stdout, &output.stderr] {
        assert!(!String::from_utf8_lossy(stream).contains("9f8e7d"));
    }
}

#[test]
fn auth_help_describes_both_commands_and_the_piped_form() {
    let assert = jevpipe().args(["auth", "--help"]).assert().success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for part in [
        "set-key",
        "remove-key",
        "OPENROUTER_API_KEY",
        "| jevpipe auth set-key",
        "keychain",
    ] {
        assert!(help.contains(part), "missing {part} in:\n{help}");
    }
    jevpipe()
        .args(["auth", "set-key", "--help"])
        .assert()
        .success()
        .stdout(contains("standard input").and(contains("hides the typing")));
}

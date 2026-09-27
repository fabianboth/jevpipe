use predicates::str::contains;

use crate::home::jevpipe;

#[test]
fn an_empty_key_is_refused_without_a_prompt_before_the_keychain_is_opened() {
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

use crate::home::{jevpipe, stdout};

const LIMITS: [&str; 5] = [
    "--request-timeout <DURATION>",
    "--max-cost <DOLLARS|none>",
    "--max-tokens <COUNT|none>",
    "--max-time <DURATION|none>",
    "3  a limit stopped the run early; standard error names the line to resume from",
];

fn help(args: &[&str]) -> String {
    stdout(jevpipe().args(args))
}

fn assert_contains(help: &str, parts: &[&str]) {
    for part in parts {
        assert!(help.contains(part), "missing {part} in:\n{help}");
    }
}

#[test]
fn filter_help_describes_every_option_and_the_exit_status() {
    let help = help(&["filter", "--help"]);

    assert_contains(
        &help,
        &[
            "Usage: jevpipe filter [OPTIONS] <QUESTION> [FILES]...",
            "<QUESTION>",
            "[FILES]...",
            "--read-files",
            "--threshold",
            "--concurrency",
            "--model",
            "1  no record was kept",
            "Examples:",
        ],
    );
    assert_contains(&help, &LIMITS);
}

#[test]
fn map_help_describes_every_option_the_questions_file_and_the_exit_status() {
    let help = help(&["map", "--help"]);

    assert_contains(
        &help,
        &[
            "Usage: jevpipe map [OPTIONS] <--questions <JSON>|--questions-file <FILE>> [FILES]...",
            "-q, --questions <JSON>",
            "-f, --questions-file <FILE>",
            "[FILES]...",
            "--read-files",
            "--concurrency",
            "--model",
            "\"type\": \"noul\"",
            "\"type\": \"choice\"",
            "\"type\": \"score\"",
            "jq",
        ],
    );
    assert_contains(&help, &LIMITS);
    assert!(!help.contains("--threshold"), "{help}");
}

#[test]
fn map_short_help_keeps_the_examples_and_leaves_out_the_formats() {
    let help = help(&["map", "-h"]);

    assert_contains(&help, &["Examples:", "-q, --questions <JSON>"]);
    assert!(!help.contains("\"type\": \"score\""), "{help}");
}

#[test]
fn config_help_names_the_commands_the_file_and_the_order() {
    assert_contains(
        &help(&["config", "--help"]),
        &[
            "list",
            "get",
            "set",
            "unset",
            "path",
            "JEVPIPE_CONFIG",
            "base-url",
            "provider",
            "typesafe",
            "https://api.typesafe.ai",
            "openrouter.max-cost",
            "[typesafe]",
            "beats",
        ],
    );
    assert_contains(
        &help(&["config", "set", "--help"]),
        &[
            "base-url, concurrency, max-cost, max-time, max-tokens, model, provider, request-timeout",
        ],
    );
}

#[test]
fn auth_help_describes_both_commands_and_the_piped_form() {
    assert_contains(
        &help(&["auth", "--help"]),
        &[
            "set-key",
            "remove-key",
            "OPENROUTER_API_KEY",
            "TYPESAFE_API_KEY",
            "configured provider",
            "keychain",
            "| jevpipe auth set-key",
        ],
    );
    assert_contains(
        &help(&["auth", "set-key", "--help"]),
        &["standard input", "hides the typing"],
    );
}

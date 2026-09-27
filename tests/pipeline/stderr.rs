use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

use crate::fixture::{json_lines, questions};
use crate::stand_in::StandIn;

const COMMANDS: [[&str; 2]; 2] = [["filter", "Is it?"], ["map", "questions.json"]];

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[tokio::test]
async fn a_failed_record_is_named_by_its_line_counting_blank_lines() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[]);

    for command in COMMANDS {
        stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .write_stdin(b"\n\na p=0.9\n   \ncaf\xe9 p=0.9\n".as_slice())
            .assert()
            .code(2)
            .stderr(contains("jevpipe: line 5: not text\n"));
    }
}

#[tokio::test]
async fn line_numbers_continue_across_files() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[("a.txt", b"a p=0.9\n\n"), ("b.txt", b"\ncaf\xe9\n")]);

    for command in COMMANDS {
        stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .args(["a.txt", "b.txt"])
            .assert()
            .code(2)
            .stderr(contains("jevpipe: line 4: not text\n"));
    }
}

#[tokio::test]
async fn a_large_failing_record_never_reaches_standard_error() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[]);
    let record = format!("toolarge {}", "x".repeat(150_000));

    for command in COMMANDS {
        let output = stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .write_stdin(format!("{record}\n"))
            .assert()
            .code(2)
            .stderr(contains("jevpipe: line 1: too large\n"));

        let stderr = stderr(output.get_output());
        assert!(!stderr.contains("xxxxxxxxxx"), "{stderr}");
        assert!(stderr.len() < 200, "{stderr}");
    }
}

#[tokio::test]
async fn inputs_that_cannot_be_read_are_named_counted_and_numbering_continues() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[
        ("a.txt", b"a p=0.9\n"),
        ("utf16.txt", b"\xff\xfea\0\n\0"),
        ("b.txt", b"\ncaf\xe9\n"),
    ]);

    for command in COMMANDS {
        stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .args(["a.txt", "gone.txt", "utf16.txt", "b.txt"])
            .assert()
            .code(2)
            .stderr(contains("jevpipe: gone.txt: not found\n"))
            .stderr(contains(
                "jevpipe: utf16.txt: UTF-16, convert it to UTF-8\n",
            ))
            .stderr(contains("jevpipe: line 3: not text\n"))
            .stderr(contains("4 records, 1 ").and(contains(", 0 skipped, 3 failed")));
    }
}

#[tokio::test]
async fn map_writes_no_line_for_an_input_that_cannot_be_read() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[("a.txt", b"a\n"), ("b.txt", b"b\n")]);

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["map", "questions.json", "a.txt", "gone.txt", "b.txt"])
        .assert()
        .code(2);

    let records: Vec<_> = json_lines(output.get_output())
        .into_iter()
        .map(|line| line["record"].clone())
        .collect();
    assert_eq!(records, ["a", "b"]);
}

#[tokio::test]
async fn the_summary_counts_truncated_files_only_when_there_are_any() {
    let stand_in = StandIn::start().await;
    let big = format!("p=0.9 {}", "a".repeat(150_000));
    let dir = questions(&[("big.txt", big.as_bytes()), ("small.txt", b"p=0.9")]);

    for command in COMMANDS {
        stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .arg("--read-files")
            .write_stdin("big.txt\nsmall.txt\n")
            .assert()
            .success()
            .stderr(contains(", 0 skipped, 0 failed, 1 truncated, $"));

        let output = stand_in
            .jevpipe()
            .current_dir(dir.path())
            .args(command)
            .arg("--read-files")
            .write_stdin("small.txt\n")
            .assert()
            .success();
        assert!(!stderr(output.get_output()).contains("truncated"));
    }
}

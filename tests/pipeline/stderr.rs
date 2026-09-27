use predicates::str::contains;

use crate::fixture::{files, json_lines, stderr};
use crate::stand_in::StandIn;

#[tokio::test]
async fn a_failed_record_is_named_by_its_line_counting_blank_lines() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .write_stdin(b"\n\na\n   \ncaf\xe9\n".as_slice())
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 5: not text\n"));
}

#[tokio::test]
async fn line_numbers_continue_across_files() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.txt", b"a p=0.9\n\n"), ("b.txt", b"\ncaf\xe9\n")]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["a.txt", "b.txt"])
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 4: not text\n"));
}

#[tokio::test]
async fn a_large_failing_record_never_reaches_standard_error() {
    let stand_in = StandIn::start().await;
    let record = format!("toolarge {}", "x".repeat(150_000));

    let output = stand_in
        .map()
        .write_stdin(format!("{record}\n"))
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 1: too large\n"));

    let stderr = stderr(output.get_output());
    assert!(!stderr.contains("xxxxxxxxxx"), "{stderr}");
    assert!(stderr.len() < 200, "{stderr}");
}

#[tokio::test]
async fn inputs_that_cannot_be_read_are_named_counted_and_numbering_continues() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("a.txt", b"a\n"),
        ("utf16.txt", b"\xff\xfea\0\n\0"),
        ("b.txt", b"\ncaf\xe9\n"),
    ]);

    let output = stand_in
        .map()
        .current_dir(dir.path())
        .args(["a.txt", "gone.txt", "utf16.txt", "b.txt"])
        .assert()
        .code(2)
        .stderr(contains("jevpipe: gone.txt: not found\n"))
        .stderr(contains(
            "jevpipe: utf16.txt: UTF-16, convert it to UTF-8\n",
        ))
        .stderr(contains("jevpipe: line 3: not text\n"))
        .stderr(contains("4 records, 1 answered, 0 skipped, 3 failed"));

    let records: Vec<_> = json_lines(output.get_output())
        .into_iter()
        .map(|line| line["record"].clone())
        .collect();
    assert_eq!(records, ["a", "caf\u{FFFD}"]);
}

#[tokio::test]
async fn the_summary_counts_truncated_files_only_when_there_are_any() {
    let stand_in = StandIn::start().await;
    let big = format!("p=0.9 {}", "a".repeat(150_000));
    let dir = files(&[("big.txt", big.as_bytes()), ("small.txt", b"p=0.9")]);

    for mut command in [stand_in.filter(), stand_in.map()] {
        command
            .current_dir(dir.path())
            .arg("--read-files")
            .write_stdin("big.txt\nsmall.txt\n")
            .assert()
            .success()
            .stderr(contains(", 0 skipped, 0 failed, 1 truncated, $"));
    }
    for mut command in [stand_in.filter(), stand_in.map()] {
        let output = command
            .current_dir(dir.path())
            .arg("--read-files")
            .write_stdin("small.txt\n")
            .assert()
            .success();
        assert!(!stderr(output.get_output()).contains("truncated"));
    }
}

#[tokio::test]
async fn a_broken_config_file_stops_the_run_before_any_request() {
    let stand_in = StandIn::start().await;
    let config = stand_in.config().display().to_string();
    let cases = [
        (
            "concurency = 4\n",
            "unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, model, request-timeout",
        ),
        (
            "concurrency = 0\n",
            "invalid value '0' for `concurrency`: number would be zero for non-zero type",
        ),
    ];
    for (settings, problem) in cases {
        stand_in.configure(settings);

        stand_in
            .filter()
            .write_stdin("a p=0.9\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains(format!("jevpipe: error: {config}: {problem}")));
    }

    assert!(stand_in.requests().await.is_empty());
}

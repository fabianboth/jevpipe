use std::time::{Duration, Instant};

use predicates::str::contains;

use crate::fixture::files;
use crate::stand_in::StandIn;

fn stderr_lines(output: &std::process::Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn keeps_lines_at_or_above_the_threshold_unchanged_and_in_input_order() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9 slow=300\nb p=0.2\nc p=0.5\nd p=0.49\ne p=0.7\n")
        .assert()
        .success()
        .stdout("a p=0.9 slow=300\nc p=0.5\ne p=0.7\n");
}

#[tokio::test]
async fn sends_the_question_and_the_line_to_the_service() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--model", "typesafe/jev-1.13"])
        .write_stdin("a p=0.9\n")
        .assert()
        .success();

    let requests = stand_in.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["model"], "typesafe/jev-1.13");
    assert_eq!(requests[0]["state"], "a p=0.9");
    assert_eq!(requests[0]["questions"]["match"]["type"], "noul");
    assert_eq!(requests[0]["questions"]["match"]["instructions"], "Is it?");
}

#[tokio::test]
async fn reads_files_in_argument_order_and_dash_as_standard_input() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.txt", b"a1 p=0.9\na2 p=0.9\n"), ("b.txt", b"b1 p=0.9\n")]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "a.txt", "-", "b.txt"])
        .write_stdin("s1 p=0.9\n")
        .assert()
        .success()
        .stdout("a1 p=0.9\na2 p=0.9\ns1 p=0.9\nb1 p=0.9\n");
}

#[tokio::test]
async fn a_higher_threshold_drops_a_line_just_below_it() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--threshold", "0.8"])
        .write_stdin("a p=0.79\nb p=0.8\n")
        .assert()
        .success()
        .stdout("b p=0.8\n");
}

#[tokio::test]
async fn exits_0_with_one_summary_line_when_something_is_kept() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9\nb p=0.1\n")
        .assert()
        .code(0);

    let stderr = stderr_lines(output.get_output());
    assert_eq!(stderr.len(), 1);
    assert!(
        stderr[0].starts_with("jevpipe: 2 records, 1 kept, 0 skipped, 0 failed, $0.00002, "),
        "{stderr:?}"
    );
    assert!(stderr[0].ends_with('s'), "{stderr:?}");
}

#[tokio::test]
async fn exits_1_when_nothing_is_kept() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.1\nb p=0.2\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(contains("2 records, 0 kept"));
}

#[tokio::test]
async fn transient_failures_are_retried_until_the_answer_arrives() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9 fail=503x2\nb p=0.9 fail=429x2\nc p=0.9 fail=524x2\nd p=0.1\n")
        .assert()
        .code(0)
        .stdout("a p=0.9 fail=503x2\nb p=0.9 fail=429x2\nc p=0.9 fail=524x2\n")
        .stderr(contains("4 records, 3 kept, 0 skipped, 0 failed"));

    assert_eq!(stand_in.requests().await.len(), 10);
}

#[tokio::test]
async fn retries_wait_only_as_long_as_the_service_asks() {
    let stand_in = StandIn::start().await;
    let started = Instant::now();

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9 fail=503x4\n")
        .assert()
        .success()
        .stdout("a p=0.9 fail=503x4\n");

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "Retry-After: 0 was not honoured: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_record_failing_after_all_retries_is_reported_and_the_others_still_judged() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9\nb p=0.9 fail=503x9\nc p=0.9\n")
        .assert()
        .code(2)
        .stdout("a p=0.9\nc p=0.9\n")
        .stderr(contains("jevpipe: line 2: service unavailable\n"))
        .stderr(contains("3 records, 2 kept, 0 skipped, 1 failed"));

    assert_eq!(stand_in.requests().await.len(), 7);
}

#[tokio::test]
async fn json_lines_and_windows_line_endings_come_out_byte_identical() {
    let stand_in = StandIn::start().await;
    let input = "{\"id\": 1, \"note\": \"p=0.9\"}\r\n{\"id\": 2, \"note\": \"p=0.1\"}\r\n{\"id\": 3, \"note\": \"p=0.9\"}\n";

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("{\"id\": 1, \"note\": \"p=0.9\"}\r\n{\"id\": 3, \"note\": \"p=0.9\"}\n");

    let states: Vec<_> = stand_in
        .requests()
        .await
        .into_iter()
        .map(|request| request["state"].clone())
        .collect();
    assert!(states.contains(&"{\"id\": 1, \"note\": \"p=0.9\"}".into()));
}

#[tokio::test]
async fn a_last_line_without_terminator_gets_one() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=0.9\nb p=0.9")
        .assert()
        .success()
        .stdout("a p=0.9\nb p=0.9\n");
}

#[tokio::test]
async fn blank_lines_are_not_records() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("\na p=0.9\n\n   \r\n\t\nb p=0.1\n\n")
        .assert()
        .success()
        .stdout("a p=0.9\n")
        .stderr(contains("2 records, 1 kept"));

    assert_eq!(stand_in.requests().await.len(), 2);
}

#[tokio::test]
async fn empty_input_exits_1_without_requests() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("")
        .assert()
        .code(1)
        .stderr(contains("jevpipe: 0 records, 0 kept, 0 skipped, 0 failed"));

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn lines_that_are_not_text_fail_without_a_request() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin(b"a p=0.9\ncaf\xe9 p=0.9\nnul\0 p=0.9\n".as_slice())
        .assert()
        .code(2)
        .stdout("a p=0.9\n")
        .stderr(contains("jevpipe: line 2: not text\n"))
        .stderr(contains("jevpipe: line 3: not text\n"))
        .stderr(contains("3 records, 1 kept, 0 skipped, 2 failed"));

    assert_eq!(stand_in.requests().await.len(), 1);
}

#[tokio::test]
async fn a_utf16_input_fails_as_a_whole_and_the_other_inputs_are_judged() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("windows.txt", b"\xff\xfeC\0a\0n\0?\0\n\0"),
        ("b.txt", b"b p=0.9\n"),
    ]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "windows.txt", "b.txt"])
        .assert()
        .code(2)
        .stdout("b p=0.9\n")
        .stderr(contains(
            "jevpipe: windows.txt: UTF-16, convert it to UTF-8\n",
        ))
        .stderr(contains("2 records, 1 kept, 0 skipped, 1 failed"));

    assert_eq!(stand_in.requests().await.len(), 1);
}

#[tokio::test]
async fn a_run_level_error_stops_the_run() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a status=401\n")
        .assert()
        .code(2)
        .stdout("");

    let stderr = stderr_lines(output.get_output());
    assert_eq!(stderr.len(), 1, "{stderr:?}");
    assert!(stderr[0].starts_with("jevpipe: error: "), "{stderr:?}");
    assert!(stderr[0].contains("No cookie auth credentials found"));
}

#[tokio::test]
async fn a_probability_outside_0_to_1_stops_the_run() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a p=2\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains(
            "jevpipe: error: service error: unexpected answer to `match`: probability 2 is not between 0 and 1",
        ));
}

#[tokio::test]
async fn an_answer_in_an_unexpected_shape_stops_the_run() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin("a malformed\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains("jevpipe: error: service error: unexpected answer"));
}

#[tokio::test]
async fn a_missing_api_key_is_named_before_any_request() {
    let stand_in = StandIn::start().await;

    for key in [None, Some("")] {
        let mut command = stand_in.jevpipe();
        match key {
            Some(value) => command.env("OPENROUTER_API_KEY", value),
            None => command.env_remove("OPENROUTER_API_KEY"),
        };
        command
            .args(["filter", "Is it?"])
            .write_stdin("a p=0.9\n")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains("jevpipe: error: OPENROUTER_API_KEY"));
    }

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn a_named_file_that_does_not_exist_fails_at_its_place_and_the_others_are_judged() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.txt", b"a p=0.9\n"), ("b.txt", b"b p=0.9\n")]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "a.txt", "gone.txt", "b.txt"])
        .assert()
        .code(2)
        .stdout("a p=0.9\nb p=0.9\n")
        .stderr(contains("jevpipe: gone.txt: not found\n"))
        .stderr(contains("3 records, 2 kept, 0 skipped, 1 failed"));
}

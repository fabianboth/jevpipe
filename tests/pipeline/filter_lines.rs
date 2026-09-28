use std::time::{Duration, Instant};

use predicates::str::contains;

use crate::fixture::{files, stderr};
use crate::stand_in::StandIn;

#[tokio::test]
async fn keeps_lines_at_or_above_the_threshold_unchanged_and_in_input_order() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
        .write_stdin("a p=0.9 slow=300\nb p=0.2\nc p=0.5\nd p=0.49\ne p=0.7\n")
        .assert()
        .success()
        .stdout("a p=0.9 slow=300\nc p=0.5\ne p=0.7\n");
}

#[tokio::test]
async fn sends_the_question_and_the_line_to_the_service() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
        .args(["--model", "typesafe/jev-1.13"])
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
        .filter()
        .current_dir(dir.path())
        .args(["a.txt", "-", "b.txt"])
        .write_stdin("s1 p=0.9\n")
        .assert()
        .success()
        .stdout("a1 p=0.9\na2 p=0.9\ns1 p=0.9\nb1 p=0.9\n");
}

#[tokio::test]
async fn a_higher_threshold_drops_a_line_just_below_it() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
        .args(["--threshold", "0.8"])
        .write_stdin("a p=0.79\nb p=0.8\n")
        .assert()
        .success()
        .stdout("b p=0.8\n");
}

#[tokio::test]
async fn exits_0_with_one_summary_line_when_something_is_kept() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .filter()
        .write_stdin("a p=0.9\nb p=0.1\n")
        .assert()
        .code(0);

    let stderr: Vec<_> = stderr(output.get_output())
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(stderr.len(), 1);
    assert!(
        stderr[0]
            .starts_with("jevpipe: 2 records, 1 kept, 0 skipped, 0 failed, 660 tokens, $0.00002, "),
        "{stderr:?}"
    );
    assert!(stderr[0].ends_with('s'), "{stderr:?}");
}

#[tokio::test]
async fn exits_1_when_nothing_is_kept() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
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
        .filter()
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
        .filter()
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
async fn retries_end_when_the_request_timeout_is_spent_and_name_the_last_failure() {
    let stand_in = StandIn::start().await;
    let started = Instant::now();

    stand_in
        .filter()
        .args(["--request-timeout", "3s"])
        .write_stdin("a p=0.9 fail=503x9 after=2\nb p=0.9\n")
        .assert()
        .code(2)
        .stdout("b p=0.9\n")
        .stderr(contains(
            "jevpipe: line 1: service unavailable (503 Service Unavailable: Provider returned error)\n",
        ));

    assert!(
        started.elapsed() < Duration::from_secs(6),
        "the retries outlasted --request-timeout 3s: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_record_failing_after_all_retries_is_reported_and_the_others_still_judged() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
        .write_stdin("a p=0.9\nb p=0.9 fail=503x9\nc p=0.9\n")
        .assert()
        .code(2)
        .stdout("a p=0.9\nc p=0.9\n")
        .stderr(contains(
            "jevpipe: line 2: service unavailable (503 Service Unavailable: Provider returned error)\n",
        ))
        .stderr(contains("3 records, 2 kept, 0 skipped, 1 failed"));

    assert_eq!(stand_in.requests().await.len(), 7);
}

#[tokio::test]
async fn json_lines_and_windows_line_endings_come_out_byte_identical() {
    let stand_in = StandIn::start().await;
    let input = "{\"id\": 1, \"note\": \"p=0.9\"}\r\n{\"id\": 2, \"note\": \"p=0.1\"}\r\n{\"id\": 3, \"note\": \"p=0.9\"}\n";

    stand_in
        .filter()
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
        .filter()
        .write_stdin("a p=0.9\nb p=0.9")
        .assert()
        .success()
        .stdout("a p=0.9\nb p=0.9\n");
}

#[tokio::test]
async fn blank_lines_are_not_records() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
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
        .filter()
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
        .filter()
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
async fn a_run_level_error_stops_the_run() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .filter()
        .write_stdin("a status=401\n")
        .assert()
        .code(2)
        .stdout("");

    let stderr: Vec<_> = stderr(output.get_output())
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(stderr.len(), 1, "{stderr:?}");
    assert!(stderr[0].starts_with("jevpipe: error: "), "{stderr:?}");
    assert!(stderr[0].contains("No cookie auth credentials found"));
}

#[tokio::test]
async fn a_probability_outside_0_to_1_stops_the_run() {
    let stand_in = StandIn::start().await;

    stand_in
        .filter()
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
        .filter()
        .write_stdin("a malformed\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains("jevpipe: error: service error: unexpected answer"));
}

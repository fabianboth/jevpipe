use predicates::str::contains;
use serde_json::{Value, json};

use crate::fixture::{QUESTIONS, files, json_lines, stderr};
use crate::stand_in::StandIn;

#[tokio::test]
async fn answers_every_question_about_each_line_in_input_order() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .map()
        .write_stdin("a p=0.9 choice=real level=2 slow=300\nb p=0.2\nc choice=flaky level=1\n")
        .assert()
        .success();

    let lines = json_lines(output.get_output());
    let records: Vec<_> = lines.iter().map(|line| &line["record"]).collect();
    assert_eq!(
        records,
        [
            "a p=0.9 choice=real level=2 slow=300",
            "b p=0.2",
            "c choice=flaky level=1"
        ]
    );
    assert_eq!(
        lines[0],
        json!({
            "record": "a p=0.9 choice=real level=2 slow=300",
            "answers": {
                "relevant": {"type": "noul", "noul": 0.9},
                "kind": {"type": "choice", "choice": "real", "probabilities": {"flaky": 0, "real": 1}, "confidence": 1},
                "severity": {
                    "type": "score", "score": 2,
                    "legend": {"0": "cosmetic", "1": "annoying", "2": "blocking"},
                    "probabilities": {"0": 0, "1": 0, "2": 1}, "confidence": 1
                }
            }
        })
    );
    assert_eq!(lines[1]["answers"]["relevant"]["noul"], 0.2);
    assert_eq!(lines[2]["answers"]["kind"]["choice"], "flaky");
    assert_eq!(lines[2]["answers"]["severity"]["score"], 1);
}

#[tokio::test]
async fn sends_one_request_per_line_with_the_questions_file_unchanged() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("questions.json", QUESTIONS.as_bytes())]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args([
            "map",
            "-f",
            "questions.json",
            "--model",
            "typesafe/jev-1.13",
        ])
        .write_stdin("a\nb\nc\n")
        .assert()
        .success();

    let asked: Value = serde_json::from_str(QUESTIONS).unwrap();
    let requests = stand_in.requests().await;
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_eq!(request["questions"], asked);
        assert_eq!(request["model"], "typesafe/jev-1.13");
    }
    let mut states: Vec<_> = requests.iter().map(|request| &request["state"]).collect();
    states.sort_by_key(ToString::to_string);
    assert_eq!(states, ["a", "b", "c"]);
}

#[tokio::test]
async fn a_json_line_is_sent_as_text_and_comes_back_as_its_text() {
    let stand_in = StandIn::start().await;
    let line = r#"{"test": "login_timeout", "note": "p=0.9"}"#;

    let output = stand_in
        .map()
        .write_stdin(format!("{line}\n"))
        .assert()
        .success();

    assert_eq!(stand_in.requests().await[0]["state"], line);
    let record = &json_lines(output.get_output())[0]["record"];
    let parsed: Value = serde_json::from_str(record.as_str().unwrap()).unwrap();
    assert_eq!(parsed["test"], "login_timeout");
}

#[tokio::test]
async fn exits_0_with_one_summary_line_when_every_record_is_answered() {
    let stand_in = StandIn::start().await;

    let output = stand_in.map().write_stdin("a\nb\nc\n").assert().code(0);

    let stderr = stderr(output.get_output());
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(
        stderr.starts_with("jevpipe: 3 records, 3 answered, 0 skipped, 0 failed, $0.00003, "),
        "{stderr}"
    );
    assert!(stderr.trim_end().ends_with('s'), "{stderr}");
}

#[tokio::test]
async fn a_record_failing_after_all_retries_gets_a_failed_line_and_the_others_are_answered() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .map()
        .write_stdin("a p=0.9\nb fail=503x9\nc\n")
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 2: service unavailable\n"))
        .stderr(contains("3 records, 2 answered, 0 skipped, 1 failed"));

    let lines = json_lines(output.get_output());
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["answers"]["relevant"]["noul"], 0.9);
    assert_eq!(
        lines[1],
        json!({"record": "b fail=503x9", "outcome": "failed", "reason": "service unavailable"})
    );
    assert_eq!(lines[2]["record"], "c");
    assert!(lines[2]["answers"].is_object());
}

#[tokio::test]
async fn a_line_that_is_not_text_gets_a_failed_line_with_the_bad_bytes_replaced() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .map()
        .write_stdin(b"caf\xe9\n".as_slice())
        .assert()
        .code(2);

    assert_eq!(
        json_lines(output.get_output()),
        [json!({"record": "caf\u{FFFD}", "outcome": "failed", "reason": "not text"})]
    );
    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn empty_input_exits_0_without_requests() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .write_stdin("\n\n")
        .assert()
        .code(0)
        .stdout("")
        .stderr(contains(
            "jevpipe: 0 records, 0 answered, 0 skipped, 0 failed",
        ));

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn an_answer_missing_a_question_stops_the_run() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .write_stdin("a malformed\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains(
            "jevpipe: error: service error: unexpected answer to `",
        ));
}

#[tokio::test]
async fn an_answer_of_another_type_than_asked_stops_the_run() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .write_stdin("a wrongtype\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains(
            "jevpipe: error: service error: unexpected answer to `kind`: a noul, asked for a choice",
        ));
}

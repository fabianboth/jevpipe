use std::fs;

use predicates::str::contains;
use serde_json::json;

use crate::fixture::{json_lines, questions};
use crate::stand_in::StandIn;

#[tokio::test]
async fn answers_each_file_with_its_path_as_the_record() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[
        ("a.rs", b"fn a() {} p=0.9 slow=300"),
        ("b.rs", b"fn b() {} choice=real"),
    ]);

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["map", "questions.json", "--read-files"])
        .write_stdin("a.rs\nb.rs\r\n")
        .assert()
        .success()
        .stderr(contains("2 records, 2 answered, 0 skipped, 0 failed"));

    let lines = json_lines(output.get_output());
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["record"], "a.rs");
    assert_eq!(lines[0]["answers"]["relevant"]["noul"], 0.9);
    assert_eq!(lines[1]["record"], "b.rs");
    assert_eq!(lines[1]["answers"]["kind"]["choice"], "real");
    assert!(lines.iter().all(|line| line.get("truncated").is_none()));
    let requests = stand_in.requests().await;
    assert!(requests.iter().any(|request| {
        request["state"] == json!({"path": "b.rs", "content": "fn b() {} choice=real"})
    }));
}

#[tokio::test]
async fn files_that_are_not_judged_get_a_skipped_line_without_a_request() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[
        ("image.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
        ("latin1.txt", b"caf\xe9"),
        ("empty.txt", b""),
    ]);
    fs::create_dir(dir.path().join("src")).unwrap();

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["map", "questions.json", "--read-files"])
        .write_stdin("image.png\nlatin1.txt\nempty.txt\nsrc\n")
        .assert()
        .code(0)
        .stderr(contains("4 records, 0 answered, 4 skipped, 0 failed"));

    assert_eq!(
        json_lines(output.get_output()),
        [
            json!({"record": "image.png", "outcome": "skipped", "reason": "binary"}),
            json!({"record": "latin1.txt", "outcome": "skipped", "reason": "binary"}),
            json!({"record": "empty.txt", "outcome": "skipped", "reason": "empty"}),
            json!({"record": "src", "outcome": "skipped", "reason": "directory"}),
        ]
    );
    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn a_missing_path_gets_a_failed_line() {
    let stand_in = StandIn::start().await;
    let dir = questions(&[("a.rs", b"fn a() {}")]);

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["map", "questions.json", "--read-files"])
        .write_stdin("a.rs\ngone.rs\n")
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 2: not found\n"));

    let lines = json_lines(output.get_output());
    assert_eq!(lines[0]["record"], "a.rs");
    assert_eq!(
        lines[1],
        json!({"record": "gone.rs", "outcome": "failed", "reason": "not found"})
    );
}

#[tokio::test]
async fn a_large_file_is_cut_to_fit_answered_and_marked_truncated() {
    let stand_in = StandIn::start().await;
    let content = format!("p=0.9 {}", "é".repeat(150_000));
    let dir = questions(&[("big.txt", content.as_bytes()), ("small.txt", b"p=0.2")]);

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["map", "questions.json", "--read-files"])
        .write_stdin("big.txt\nsmall.txt\n")
        .assert()
        .success();

    let lines = json_lines(output.get_output());
    assert_eq!(lines[0]["record"], "big.txt");
    assert_eq!(lines[0]["truncated"], true);
    assert_eq!(lines[0]["answers"]["relevant"]["noul"], 0.9);
    assert_eq!(lines[1]["record"], "small.txt");
    assert!(lines[1].get("truncated").is_none(), "{}", lines[1]);
    let requests = stand_in.requests().await;
    let sent = requests
        .iter()
        .find_map(|request| {
            (request["state"]["path"] == "big.txt").then(|| request["state"]["content"].clone())
        })
        .unwrap();
    assert_eq!(sent.as_str().unwrap().chars().count(), 100_000);
}

use predicates::str::contains;
use serde_json::{Value, json};

use crate::fixture::files;
use crate::stand_in::StandIn;

fn json_lines(output: &std::process::Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[tokio::test]
async fn json_prints_kept_records_as_objects_in_input_order() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--json"])
        .write_stdin("a p=0.9 slow=300\nb p=0.2\nc p=0.6\n")
        .assert()
        .code(0)
        .stderr(contains("3 records, 2 kept, 0 skipped, 0 failed"));

    assert_eq!(
        json_lines(output.get_output()),
        [
            json!({"position": 1, "record": "a p=0.9 slow=300", "outcome": "kept", "probability": 0.9, "truncated": false}),
            json!({"position": 3, "record": "c p=0.6", "outcome": "kept", "probability": 0.6, "truncated": false}),
        ]
    );
}

#[tokio::test]
async fn json_all_prints_every_record_with_its_outcome() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("kept.rs", b"p=0.9"),
        ("dropped.rs", b"p=0.3"),
        ("image.png", b"\0\0"),
    ]);

    let output = stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files", "--json", "--all"])
        .write_stdin("dropped.rs\nimage.png\nkept.rs\ngone.rs\n")
        .assert()
        .code(2)
        .stderr(contains("jevpipe: record 4 (gone.rs): not found\n"))
        .stderr(contains("4 records, 1 kept, 1 skipped, 1 failed"));

    assert_eq!(
        json_lines(output.get_output()),
        [
            json!({"position": 1, "record": "dropped.rs", "outcome": "dropped", "probability": 0.3, "truncated": false}),
            json!({"position": 2, "record": "image.png", "outcome": "skipped", "reason": "binary"}),
            json!({"position": 3, "record": "kept.rs", "outcome": "kept", "probability": 0.9, "truncated": false}),
            json!({"position": 4, "record": "gone.rs", "outcome": "failed", "reason": "not found"}),
        ]
    );
}

#[tokio::test]
async fn json_keeps_the_exit_status_of_a_run_where_nothing_is_kept() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--json"])
        .write_stdin("a p=0.1\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(contains("1 records, 0 kept"));
}

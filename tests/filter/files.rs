use std::fs;

use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

use crate::fixture::files;
use crate::stand_in::StandIn;

#[tokio::test]
async fn prints_the_paths_of_matching_files_in_input_order() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("a.rs", b"fn a() {} p=0.9 slow=300"),
        ("b.rs", b"fn b() {} p=0.1"),
        ("c.rs", b"fn c() {} p=0.9"),
    ]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("a.rs\nb.rs\r\nc.rs\n")
        .assert()
        .success()
        .stdout("a.rs\nc.rs\n");

    let requests = stand_in.requests().await;
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().any(|request| {
        request["state"]["path"] == "b.rs" && request["state"]["content"] == "fn b() {} p=0.1"
    }));
}

#[tokio::test]
async fn binary_and_non_utf8_files_are_skipped_without_a_request() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("text.rs", b"fn a() {} p=0.9"),
        ("image.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR p=0.9"),
        ("latin1.txt", b"caf\xe9 p=0.9"),
    ]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("text.rs\nimage.png\nlatin1.txt\n")
        .assert()
        .code(0)
        .stdout("text.rs\n")
        .stderr(contains("3 records, 1 kept, 2 skipped, 0 failed"));

    assert_eq!(stand_in.requests().await.len(), 1);
}

#[tokio::test]
async fn empty_files_and_directories_are_skipped() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("empty.txt", b"")]);
    fs::create_dir(dir.path().join("src")).unwrap();

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("empty.txt\nsrc\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(contains("2 records, 0 kept, 2 skipped, 0 failed"));

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn a_missing_path_is_reported_and_fails() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.rs", b"fn a() {} p=0.9")]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("a.rs\ngone.rs\n")
        .assert()
        .code(2)
        .stdout("a.rs\n")
        .stderr(contains("jevpipe: record 2 (gone.rs): not found\n"))
        .stderr(contains("2 records, 1 kept, 0 skipped, 1 failed"));
}

#[tokio::test]
async fn a_large_file_is_cut_to_fit_judged_and_marked_truncated() {
    let stand_in = StandIn::start().await;
    let content = format!("p=0.9 {}", "é".repeat(150_000));
    let dir = files(&[("big.txt", content.as_bytes())]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files", "--json"])
        .write_stdin("big.txt\n")
        .assert()
        .success()
        .stdout(contains("\"record\":\"big.txt\"").and(contains("\"truncated\":true")));

    let requests = stand_in.requests().await;
    let sent = requests[0]["state"]["content"].as_str().unwrap();
    assert_eq!(sent.chars().count(), 100_000);
    assert!(content.starts_with(sent));
}

#[tokio::test]
async fn a_413_answer_fails_the_record_as_too_large_and_the_run_continues() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("huge.txt", b"status=413"), ("a.rs", b"p=0.9")]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("huge.txt\na.rs\n")
        .assert()
        .code(2)
        .stdout("a.rs\n")
        .stderr(contains("jevpipe: record 1 (huge.txt): too large\n"));
}

#[tokio::test]
async fn a_file_the_service_finds_too_large_fails() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("dense.min.js", b"toolarge")]);

    stand_in
        .jevpipe()
        .current_dir(dir.path())
        .args(["filter", "Is it?", "--read-files"])
        .write_stdin("dense.min.js\n")
        .assert()
        .code(2)
        .stderr(contains("jevpipe: record 1 (dense.min.js): too large\n"));
}

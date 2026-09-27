use std::fs;

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
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
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
async fn a_utf8_byte_order_mark_is_not_sent() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("bom.rs", b"\xef\xbb\xbffn a() {} p=0.9")]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("bom.rs\n")
        .assert()
        .success();

    assert_eq!(
        stand_in.requests().await[0]["state"]["content"],
        "fn a() {} p=0.9"
    );
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
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("text.rs\nimage.png\nlatin1.txt\n")
        .assert()
        .code(0)
        .stdout("text.rs\n")
        .stderr(contains("3 records, 1 kept, 2 skipped, 0 failed"));

    assert_eq!(stand_in.requests().await.len(), 1);
}

#[tokio::test]
async fn utf16_files_are_decoded_and_judged() {
    let stand_in = StandIn::start().await;
    let text = "p=0.9 Can we ship on Friday? \u{1F680}";
    let little_endian: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let big_endian: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    let dir = files(&[("le.txt", &little_endian), ("be.txt", &big_endian)]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("le.txt\nbe.txt\n")
        .assert()
        .success()
        .stdout("le.txt\nbe.txt\n");

    let requests = stand_in.requests().await;
    assert!(
        requests
            .iter()
            .all(|request| request["state"]["content"] == text)
    );
}

#[tokio::test]
async fn broken_utf16_files_are_skipped_as_binary() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("odd.txt", b"\xff\xfea\0b"),
        ("zero.txt", b"\xff\xfea\0\0\0b\0"),
    ]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("odd.txt\nzero.txt\n")
        .assert()
        .code(1)
        .stderr(contains("2 records, 0 kept, 2 skipped, 0 failed"));

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn empty_files_and_directories_are_skipped() {
    let stand_in = StandIn::start().await;
    let dir = files(&[
        ("empty.txt", b""),
        ("utf8-bom-only.txt", b"\xef\xbb\xbf"),
        ("utf16-bom-only.txt", b"\xff\xfe"),
    ]);
    fs::create_dir(dir.path().join("src")).unwrap();

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("empty.txt\nutf8-bom-only.txt\nutf16-bom-only.txt\nsrc\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(contains("4 records, 0 kept, 4 skipped, 0 failed"));

    assert!(stand_in.requests().await.is_empty());
}

#[tokio::test]
async fn a_missing_path_is_reported_and_fails() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.rs", b"fn a() {} p=0.9")]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("a.rs\ngone.rs\n")
        .assert()
        .code(2)
        .stdout("a.rs\n")
        .stderr(contains("jevpipe: line 2: not found\n"))
        .stderr(contains("2 records, 1 kept, 0 skipped, 1 failed"));
}

#[tokio::test]
async fn a_413_answer_fails_the_record_as_too_large_and_the_run_continues() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("huge.txt", b"status=413"), ("a.rs", b"p=0.9")]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("huge.txt\na.rs\n")
        .assert()
        .code(2)
        .stdout("a.rs\n")
        .stderr(contains("jevpipe: line 1: too large\n"));
}

#[tokio::test]
async fn a_file_the_service_finds_too_large_fails() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("dense.min.js", b"toolarge")]);

    stand_in
        .filter()
        .current_dir(dir.path())
        .args(["--read-files"])
        .write_stdin("dense.min.js\n")
        .assert()
        .code(2)
        .stderr(contains("jevpipe: line 1: too large\n"));
}

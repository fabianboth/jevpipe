use std::io::{BufRead, BufReader, Read, Write};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use predicates::str::contains;

use crate::stand_in::StandIn;

#[tokio::test]
async fn writes_a_kept_record_before_the_input_ends() {
    let stand_in = StandIn::start().await;
    let mut child = stand_in
        .command()
        .args(["filter", "Is it?"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).unwrap();
        sender.send(line).unwrap();
    });

    stdin.write_all(b"a p=0.9\n").unwrap();
    stdin.flush().unwrap();
    let line = receiver.recv_timeout(Duration::from_secs(10));
    drop(stdin);

    assert_eq!(line.unwrap(), "a p=0.9\n");
    assert!(child.wait().unwrap().success());
}

#[tokio::test]
async fn stops_quietly_when_the_reader_goes_away() {
    let stand_in = StandIn::start().await;
    let records = 2000;
    let mut child = stand_in
        .command()
        .args(["filter", "Is it?"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    thread::spawn(move || {
        for record in 0..records {
            if writeln!(stdin, "{record} p=0.9 slow=200").is_err() {
                break;
            }
        }
    });

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    stdout.read_line(&mut first).unwrap();
    drop(stdout);
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let status = child.wait().unwrap();

    assert_eq!(first, "0 p=0.9 slow=200\n");
    assert!(status.success(), "{status:?}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("jevpipe: "), "{stderr}");
    assert!(stand_in.requests().await.len() < records);
}

#[tokio::test]
async fn decides_many_records_concurrently() {
    let stand_in = StandIn::start().await;
    let lines: Vec<_> = (0..200)
        .map(|record| format!("{record} p=0.9 slow=300\n"))
        .collect();
    let input = lines.concat();
    let started = Instant::now();

    stand_in
        .jevpipe()
        .args(["filter", "Is it?"])
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(input);

    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn concurrency_limits_the_requests_in_flight() {
    let stand_in = StandIn::start().await;
    let input = "a p=0.9 slow=500\nb p=0.9 slow=500\nc p=0.9 slow=500\nd p=0.9 slow=500\n";
    let started = Instant::now();

    stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--concurrency", "2"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(input);

    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "4 slow records at concurrency 2 need two rounds: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_request_that_hangs_is_abandoned_and_retried() {
    let stand_in = StandIn::start().await;

    stand_in
        .jevpipe()
        .args(["filter", "Is it?", "--request-timeout", "1"])
        .write_stdin("a p=0.9 slow=1500\n")
        .assert()
        .success()
        .stdout("a p=0.9 slow=1500\n")
        .stderr(contains("1 records, 1 kept"));

    assert_eq!(stand_in.requests().await.len(), 2);
}

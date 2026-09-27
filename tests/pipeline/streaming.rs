use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use predicates::str::contains;
use serde_json::Value;

use crate::fixture::QUESTIONS;
use crate::stand_in::StandIn;

struct Running(Child);

impl Running {
    fn map(stand_in: &StandIn) -> Self {
        let child = stand_in
            .command()
            .args(["map", "-q", QUESTIONS])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Self(child)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn map_answers_each_step_before_the_next_line_is_written() {
    let stand_in = StandIn::start().await;
    let mut running = Running::map(&stand_in);
    let mut stdin = running.0.stdin.take().unwrap();
    let stdout = running.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line.unwrap()).is_err() {
                break;
            }
        }
    });

    for step in ["step one p=0.9", "step two p=0.2"] {
        writeln!(stdin, "{step}").unwrap();
        stdin.flush().unwrap();
        let line = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        let answer: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(answer["record"], step);
        assert!(answer["answers"]["kind"]["choice"].is_string(), "{answer}");
    }
    drop(stdin);

    assert!(running.0.wait().unwrap().success());
}

#[tokio::test]
async fn map_stops_quietly_when_the_reader_goes_away() {
    let stand_in = StandIn::start().await;
    let records = 2000;
    let mut running = Running::map(&stand_in);
    let mut stdin = running.0.stdin.take().unwrap();
    thread::spawn(move || {
        for record in 0..records {
            if writeln!(stdin, "{record} slow=200").is_err() {
                break;
            }
        }
    });

    let mut stdout = BufReader::new(running.0.stdout.take().unwrap());
    let mut first = String::new();
    stdout.read_line(&mut first).unwrap();
    drop(stdout);
    let mut stderr = String::new();
    running
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let status = running.0.wait().unwrap();

    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["record"], "0 slow=200");
    assert!(status.success(), "{status:?}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stand_in.requests().await.len() < records);
}

#[tokio::test]
async fn map_answers_many_records_concurrently() {
    let stand_in = StandIn::start().await;
    let lines: Vec<_> = (0..200)
        .map(|record| format!("{record} slow=300\n"))
        .collect();
    let started = Instant::now();

    stand_in
        .map()
        .write_stdin(lines.concat())
        .assert()
        .success()
        .stderr(contains("200 records, 200 answered"));

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
        .filter()
        .args(["--concurrency", "2"])
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
        .filter()
        .args(["--request-timeout", "1s"])
        .write_stdin("a p=0.9 slow=1500\n")
        .assert()
        .success()
        .stdout("a p=0.9 slow=1500\n")
        .stderr(contains("1 records, 1 kept"));

    assert_eq!(stand_in.requests().await.len(), 2);
}

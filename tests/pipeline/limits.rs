use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use predicates::str::contains;

use crate::fixture::{files, json_lines, stderr};
use crate::stand_in::StandIn;

fn lines(count: usize, markers: &str) -> String {
    let lines: Vec<_> = (1..=count)
        .map(|line| format!("r{line} {markers}\n"))
        .collect();
    lines.concat()
}

fn records(output: &std::process::Output) -> Vec<String> {
    json_lines(output)
        .into_iter()
        .map(|line| line["record"].as_str().unwrap().to_owned())
        .collect()
}

fn resume_line(output: &std::process::Output) -> usize {
    let stderr = stderr(output);
    let (_, after) = stderr.split_once("input from line ").unwrap();
    after.split_whitespace().next().unwrap().parse().unwrap()
}

#[tokio::test]
async fn the_spend_limit_stops_sending_and_names_the_resume_line() {
    let stand_in = StandIn::start().await;
    let input = lines(100, "cost=0.0001");

    let output = stand_in
        .map()
        .args(["--concurrency", "1", "--max-cost", "0.001"])
        .write_stdin(input.clone())
        .assert()
        .code(3)
        .stderr(contains(
            "jevpipe: stopped: spend limit $0.001 reached ($0.001 spent); input from line 11 on was not processed\n",
        ));

    let expected: Vec<_> = input.lines().take(10).map(str::to_owned).collect();
    assert_eq!(records(output.get_output()), expected);
    assert_eq!(stand_in.requests().await.len(), 10);
}

#[tokio::test]
async fn answers_in_flight_at_the_limit_are_printed_and_paid_for() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .map()
        .args(["--concurrency", "5", "--max-cost", "0.001"])
        .write_stdin(lines(20, "cost=0.001 slow=300"))
        .assert()
        .code(3)
        .stderr(contains(
            "jevpipe: stopped: spend limit $0.001 reached ($0.005 spent); input from line 6 on was not processed\n",
        ))
        .stderr(contains("5 records, 5 answered, 0 skipped, 0 failed, $0.005, "));

    assert_eq!(
        records(output.get_output()),
        [
            "r1 cost=0.001 slow=300",
            "r2 cost=0.001 slow=300",
            "r3 cost=0.001 slow=300",
            "r4 cost=0.001 slow=300",
            "r5 cost=0.001 slow=300"
        ]
    );
    assert_eq!(stand_in.requests().await.len(), 5);
}

#[tokio::test]
async fn the_time_limit_abandons_requests_in_flight() {
    let stand_in = StandIn::start().await;
    let started = Instant::now();

    let output = stand_in
        .filter()
        .args(["--concurrency", "1", "--max-time", "2s"])
        .write_stdin(lines(10, "p=0.9 slow=1000"))
        .assert()
        .code(3)
        .stderr(contains(
            "jevpipe: stopped: time limit 2s reached; input from line ",
        ));

    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    let kept = String::from_utf8_lossy(&output.get_output().stdout)
        .lines()
        .count();
    assert!(kept < 10, "{kept}");
    assert_eq!(resume_line(output.get_output()), kept + 1);
}

#[tokio::test]
async fn the_time_limit_stops_a_run_waiting_for_input() {
    let stand_in = StandIn::start().await;
    let started = Instant::now();
    let mut child = stand_in
        .command()
        .args(["filter", "Is it?", "--max-time", "1s"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _open_input = child.stdin.take();

    while child.try_wait().unwrap().is_none() && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains(
            "jevpipe: stopped: time limit 1s reached; input from line 1 on was not processed\n"
        ),
        "{}",
        stderr(&output)
    );
}

#[tokio::test]
async fn a_rerun_from_the_resume_line_completes_the_output() {
    let stand_in = StandIn::start().await;
    let first = "r1 p=0.9 cost=0.001\n\nr2 p=0.9 cost=0.001\n";
    let second = "\nr3 p=0.9 cost=0.001\n\n\nr4 p=0.2 cost=0.001\nr5 p=0.9 cost=0.001\n";
    let dir = files(&[("a.txt", first.as_bytes()), ("b.txt", second.as_bytes())]);
    let joined = format!("{first}{second}");

    for command in [StandIn::filter, StandIn::map] {
        let whole = command(&stand_in)
            .write_stdin(joined.clone())
            .output()
            .unwrap();
        let stopped = command(&stand_in)
            .current_dir(dir.path())
            .args([
                "--concurrency",
                "1",
                "--max-cost",
                "0.002",
                "a.txt",
                "b.txt",
            ])
            .output()
            .unwrap();
        assert_eq!(stopped.status.code(), Some(3), "{}", stderr(&stopped));
        let resume = resume_line(&stopped);
        assert_eq!(resume, 4);
        let rest: String = joined.split_inclusive('\n').skip(resume - 1).collect();
        let rerun = command(&stand_in).write_stdin(rest).output().unwrap();

        assert_eq!([stopped.stdout, rerun.stdout].concat(), whole.stdout);
    }
}

#[tokio::test]
async fn a_used_up_key_limit_or_credits_stop_the_run() {
    let stand_in = StandIn::start().await;
    let cases = [
        ("key_limit", "the API key's spend limit is used up"),
        ("credits", "the OpenRouter account's credits are used up"),
    ];
    for (limit, reason) in cases {
        let output = stand_in
            .map()
            .args(["--concurrency", "1"])
            .write_stdin(format!("a\nb limit={limit}\nc\n"))
            .assert()
            .code(3)
            .stderr(contains(format!(
                "jevpipe: stopped: {reason}; input from line 2 on was not processed\n"
            )));

        assert_eq!(records(output.get_output()), ["a"]);
    }
}

#[tokio::test]
async fn a_full_in_flight_budget_is_retried() {
    let stand_in = StandIn::start().await;

    let output = stand_in
        .map()
        .write_stdin("a inflight=2\n")
        .assert()
        .success();

    assert_eq!(records(output.get_output()), ["a inflight=2"]);
    assert!(!stderr(output.get_output()).contains("stopped"));
    assert_eq!(stand_in.requests().await.len(), 3);
}

#[tokio::test]
async fn payment_required_without_a_limit_source_is_a_run_level_error() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .write_stdin("a status=402\n")
        .assert()
        .code(2)
        .stderr(contains(
            "jevpipe: error: service error: 402 Payment Required",
        ));
}

#[tokio::test]
async fn a_spend_limit_needs_the_service_to_report_costs() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .args(["--max-cost", "1"])
        .write_stdin("a nocost\n")
        .assert()
        .code(2)
        .stderr(contains(
            "jevpipe: error: the service reported no cost, so --max-cost cannot be enforced\n",
        ));
}

#[tokio::test]
async fn limits_that_are_not_reached_change_nothing() {
    let stand_in = StandIn::start().await;

    for limits in [
        ["--max-cost", "1", "--max-time", "1h"],
        ["--max-cost", "none", "--max-time", "none"],
    ] {
        let output = stand_in
            .filter()
            .args(limits)
            .write_stdin("a p=0.9\nb p=0.1\n")
            .assert()
            .success()
            .stdout("a p=0.9\n");

        assert!(!stderr(output.get_output()).contains("stopped"));
    }
}

#[tokio::test]
async fn records_decided_without_a_request_before_the_stop_are_printed() {
    let stand_in = StandIn::start().await;
    let dir = files(&[("a.txt", b"a cost=0.001"), ("b.txt", b"b cost=0.001")]);
    std::fs::create_dir(dir.path().join("sub")).unwrap();

    let output = stand_in
        .map()
        .current_dir(dir.path())
        .args(["--read-files", "--concurrency", "1", "--max-cost", "0.001"])
        .write_stdin("a.txt\nsub\nb.txt\n")
        .assert()
        .code(3)
        .stderr(contains("input from line 3 on was not processed"));

    let lines = json_lines(output.get_output());
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1]["outcome"], "skipped");
}

#[tokio::test]
async fn a_stop_outranks_failed_records() {
    let stand_in = StandIn::start().await;

    stand_in
        .map()
        .args(["--concurrency", "1", "--max-cost", "0.001"])
        .write_stdin(b"caf\xe9\na cost=0.001\nb\n".as_slice())
        .assert()
        .code(3)
        .stderr(contains("jevpipe: line 1: not text\n"))
        .stderr(contains("input from line 3 on was not processed"));
}

use predicates::str::contains;

use crate::stand_in::StandIn;

#[tokio::test]
async fn configured_settings_apply_to_runs_without_flags() {
    let stand_in = StandIn::start().await;
    for (key, value) in [
        ("model", "typesafe/jev-1.13"),
        ("concurrency", "1"),
        ("request-timeout", "20s"),
        ("max-cost", "0.002"),
        ("max-time", "1h"),
    ] {
        stand_in
            .jevpipe()
            .args(["config", "set", key, value])
            .assert()
            .success();
    }

    stand_in
        .filter()
        .write_stdin("a p=0.9 cost=0.001\nb p=0.9 cost=0.001\nc p=0.9 cost=0.001\n")
        .assert()
        .code(3)
        .stdout("a p=0.9 cost=0.001\nb p=0.9 cost=0.001\n")
        .stderr(contains("input from line 3 on was not processed"));

    let requests = stand_in.requests().await;
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| request["model"] == "typesafe/jev-1.13")
    );
}

#[tokio::test]
async fn a_flag_beats_the_config_file_for_one_run() {
    let stand_in = StandIn::start().await;
    stand_in.configure("model = \"typesafe/jev-1.13\"\nmax-cost = 0.0001\n");

    stand_in
        .filter()
        .args(["--model", "typesafe/jev-1.14", "--max-cost", "none"])
        .write_stdin("a p=0.9 cost=0.001\nb p=0.9 cost=0.001\n")
        .assert()
        .success()
        .stdout("a p=0.9 cost=0.001\nb p=0.9 cost=0.001\n");

    let requests = stand_in.requests().await;
    assert!(
        requests
            .iter()
            .all(|request| request["model"] == "typesafe/jev-1.14")
    );
}

use predicates::str::contains;

use crate::stand_in::{OPENROUTER_KEY, StandIn, TYPESAFE_KEY};

const LINES: &str = "a p=0.9\nb p=0.2\nc p=0.7\n";

#[tokio::test]
async fn a_typesafe_run_sends_the_typesafe_key_and_model_and_decides_the_same() {
    let openrouter = StandIn::start().await;
    let typesafe = StandIn::typesafe().await;

    let expected = openrouter.filter().write_stdin(LINES).output().unwrap();
    typesafe
        .filter()
        .env("OPENROUTER_API_KEY", OPENROUTER_KEY)
        .write_stdin(LINES)
        .assert()
        .success()
        .stdout(String::from_utf8(expected.stdout).unwrap());

    assert_eq!(
        typesafe.keys().await,
        vec![format!("Bearer {TYPESAFE_KEY}"); 3]
    );
    assert!(
        typesafe
            .requests()
            .await
            .iter()
            .all(|request| request["model"] == "jev-latest")
    );
}

#[tokio::test]
async fn a_typesafe_map_run_prints_the_same_answers() {
    let openrouter = StandIn::start().await;
    let typesafe = StandIn::typesafe().await;

    let expected = openrouter.map().write_stdin(LINES).output().unwrap();
    typesafe
        .map()
        .write_stdin(LINES)
        .assert()
        .success()
        .stdout(String::from_utf8(expected.stdout).unwrap());
}

#[tokio::test]
async fn an_openrouter_run_never_sends_the_typesafe_key() {
    let openrouter = StandIn::start().await;

    openrouter
        .filter()
        .env("TYPESAFE_API_KEY", TYPESAFE_KEY)
        .write_stdin("a p=0.9\n")
        .assert()
        .success();

    assert_eq!(
        openrouter.keys().await,
        [format!("Bearer {OPENROUTER_KEY}")]
    );
}

#[tokio::test]
async fn a_pinned_model_is_sent_as_given() {
    let typesafe = StandIn::typesafe().await;

    typesafe
        .filter()
        .args(["--model", "jev-1.13.0"])
        .write_stdin("a p=0.9\n")
        .assert()
        .success();

    assert_eq!(typesafe.requests().await[0]["model"], "jev-1.13.0");
}

#[tokio::test]
async fn a_typesafe_refusal_shows_the_services_own_message() {
    let typesafe = StandIn::typesafe().await;

    typesafe
        .filter()
        .args(["--model", "jev-1.13"])
        .write_stdin("a p=0.9\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr("jevpipe: error: service error: 400 Bad Request: Unknown model: jev-1.13\n");

    typesafe
        .filter()
        .write_stdin("a errortype\n")
        .assert()
        .code(2)
        .stderr("jevpipe: error: service error: 400 Bad Request: api_usage_error\n");

    typesafe
        .filter()
        .write_stdin("a validation\n")
        .assert()
        .code(2)
        .stderr(contains("service error: 422 Unprocessable Entity\n"));
}

#[tokio::test]
async fn each_provider_asks_the_model_of_its_own_section() {
    let settings = "[openrouter]\nmodel = \"jev-1.13\"\n\n[typesafe]\nmodel = \"jev-1.13.0\"\n";
    for (stand_in, model) in [
        (StandIn::start().await, "jev-1.13"),
        (StandIn::typesafe().await, "jev-1.13.0"),
    ] {
        stand_in.configure(settings);

        stand_in
            .filter()
            .write_stdin("a p=0.9\n")
            .assert()
            .success();

        assert_eq!(stand_in.requests().await[0]["model"], model);
    }
}

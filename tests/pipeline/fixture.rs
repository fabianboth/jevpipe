use std::fs;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

pub(crate) fn files(entries: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, content) in entries {
        fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}

pub(crate) fn json_lines(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub(crate) const QUESTIONS: &str = r#"{
  "relevant": {"type": "noul", "instructions": "Is this failure worth a closer look?"},
  "kind": {"type": "choice", "instructions": "What kind of failure is this?",
           "criteria": {"flaky": "infra or timing", "real": "deterministic bug"}},
  "severity": {"type": "score", "instructions": "How severe is this failure?",
               "criteria": ["cosmetic", "annoying", "blocking"]}
}"#;

pub(crate) fn questions(entries: &[(&str, &[u8])]) -> TempDir {
    let dir = files(entries);
    fs::write(dir.path().join("questions.json"), QUESTIONS).unwrap();
    dir
}

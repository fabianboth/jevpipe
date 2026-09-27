# Quickstart: Map Command

## Verify (offline, no API key)

```powershell
./check.ps1 -Fix
./check.ps1 -Filter map
```

## Try it against the real service

Needs `OPENROUTER_API_KEY` in the environment.

```bash
cargo build --release
cat > triage.json <<'EOF'
{
  "relevant": {"type": "noul", "instructions": "Is this failure worth a closer look?"},
  "kind": {"type": "choice", "instructions": "What kind of failure is this?",
           "criteria": {"flaky": "infra or timing", "real": "deterministic bug"}},
  "severity": {"type": "score", "instructions": "How severe is this failure?",
               "criteria": ["cosmetic", "annoying", "blocking"]}
}
EOF

# label every record, keep only what the script needs
./target/release/jevpipe map -f triage.json failures.jsonl \
  | jq -c 'select(.answers.kind.choice == "flaky") | .record | fromjson | .test'

# several questions about each file in one call
git ls-files | ./target/release/jevpipe map -f questions.json --read-files | jq -c '{record, answers}'

# selection and labels in one pass: put the yes/no question into the file, select with jq
./target/release/jevpipe map -f triage.json failures.jsonl | jq -c 'select(.answers.relevant.noul >= 0.8)'

# one step of a loop: one state in, one answer out
echo "$PAGE_STATE" | ./target/release/jevpipe map -q "$STEP_QUESTIONS"

# inline questions: no file to write
./target/release/jevpipe map -q '{"error": {"type": "noul", "instructions": "Is this line an error?"}}' app.log

# filter is unchanged, apart from --json/--all being gone
git ls-files | ./target/release/jevpipe filter "Does this file define command line arguments?" --read-files
```

Check after each run: the summary line on stderr (no record content on it), and `echo $?` (`map`: 0
nothing failed, 2 something failed).

## Checked against the real service (2026-09-27)

Every command above ran against `typesafe/jev-1.13-20260917` through OpenRouter. Three records with
three questions cost $0.000052 in 0.7 s; one step of the loop took 0.4 s end to end. The answers came
back in the documented shape and were passed through unchanged (the service orders a choice's
`probabilities` by value, not by the questions file). No surprises.

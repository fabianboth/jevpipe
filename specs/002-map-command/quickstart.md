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
./target/release/jevpipe map triage.json failures.jsonl \
  | jq -c 'select(.answers.kind.choice == "flaky") | .record | fromjson | .test'

# several questions about each file in one call
git ls-files | ./target/release/jevpipe map questions.json --read-files | jq -c '{record, answers}'

# selection and labels in one pass: put the yes/no question into the file, select with jq
./target/release/jevpipe map triage.json failures.jsonl | jq -c 'select(.answers.relevant.noul >= 0.8)'

# one step of a loop: one state in, one answer out
echo "$PAGE_STATE" | ./target/release/jevpipe map step.json

# filter is unchanged, apart from --json/--all being gone
git ls-files | ./target/release/jevpipe filter "Does this file define command line arguments?" --read-files
```

Check after each run: the summary line on stderr (no record content on it), and `echo $?` (`map`: 0
nothing failed, 2 something failed).

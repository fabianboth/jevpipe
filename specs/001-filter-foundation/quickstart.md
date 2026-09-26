# Quickstart: Filter Foundation

## Verify (offline, no API key)

```powershell
./check.ps1 -Fix
./check.ps1 -Filter filter
```

## Try it against the real service

Needs `OPENROUTER_API_KEY` in the environment.

```bash
cargo build --release

# semantic grep over this repository
git ls-files | ./target/release/jevpipe filter "Does this file define command line arguments?" --read-files

# see how well the question separates the files
git ls-files | ./target/release/jevpipe filter "Does this file define command line arguments?" --read-files --json --all | jq -c '{record, outcome, probability}'

# lines from a file, stricter threshold
./target/release/jevpipe filter "Is this a question?" --threshold 0.8 notes.txt

# JSONL records pass through unchanged
./target/release/jevpipe filter "Is this failure caused by timing or infrastructure?" failures.jsonl | jq .test

# stops cleanly when the reader is done
git ls-files | ./target/release/jevpipe filter "Is this Rust code?" --read-files | head -2
```

Check after each run: the summary line on stderr, and `echo $?` (0 kept something, 1 kept nothing,
2 something failed).

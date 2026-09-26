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

## Observed on the first real run (2026-09-26)

- The repository's 88 tracked files took 2.4–3.4 s and cost about $0.007 per question.
- `git ls-files` lists only committed files; add `--others --exclude-standard` to include new ones.
- "Does this file define command line arguments?" kept `src/cli.rs` (0.99) but also every shell and
  PowerShell script with a `param` block (0.96–0.99) and the specs describing the CLI. The answers
  were right, but the question was broader than intended: check `--json --all` before trusting one.
- "Is this Rust code?" gave 0.97–0.99 for every `.rs` file and also kept `Cargo.toml`.

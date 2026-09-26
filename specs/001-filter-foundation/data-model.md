# Data Model: Filter Foundation

jevpipe keeps no stored data. These are the in-memory types a run flows through:
`Record` → `Decision` → output line, with every `Decision` counted into the `Summary`.

## Record

One unit of input.

| Field | Meaning |
|---|---|
| position | 1-based index among all records of the run, across all inputs; blank lines are not counted |
| raw | the original bytes of the line, including its line terminator; what the default output writes back |
| content | what gets judged: the line's text, or the file named by it (`--read-files`) |

Validation (a failed validation makes the record's decision `failed`):

- Text: the line without its terminator must be valid UTF-8 without NUL bytes (else `not text`).
- Line mode: an input that starts with a UTF-16 byte-order mark fails as one record (`UTF-16,
  convert it to UTF-8`).
- A file named on the command line that cannot be opened becomes one failed record at its place, with the file name as its content.
- `--read-files`: the path must exist and be readable (else failed). A file that starts with a UTF-16
  byte-order mark is decoded as UTF-16, like ripgrep does (the mark takes precedence over the NUL
  check, since UTF-16 text is full of zero bytes); it is `skipped` as binary when it does not decode
  or contains U+0000. Other files are `skipped` when they are directories, empty, have a NUL byte in
  the first 8 KiB, or are not valid UTF-8.

## State (sent to the service)

| Mode | State |
|---|---|
| default | the line as a string |
| `--read-files` | `{"path": <path>, "content": <text, at most 100,000 characters>}` |

## Decision

The outcome for one record. Exactly one of:

| Outcome | Carries | Output |
|---|---|---|
| kept | probability ≥ threshold | default, `--json`, `--all` |
| dropped | probability < threshold | `--all` only |
| skipped | reason (`directory`, `empty`, `binary`) | `--all` only |
| failed | reason (for example `not found`, `not text`, `UTF-16, convert it to UTF-8`, `too large`, `service unavailable`) | `--all` only, plus one stderr line |

Every decision also carries `truncated` (only possible with `--read-files`).

Transitions: a record starts undecided; reading or validation can end it as skipped or failed; a
service answer ends it as kept or dropped; a record-level service error or exhausted retries end it as
failed. A run-level error ends the whole run instead (no decision).

## Summary

| Field | Meaning |
|---|---|
| records, kept, skipped, failed | counts; `dropped = records − kept − skipped − failed` |
| cost | sum of `usage.cost` from all answers (omitted if the service reports none) |
| duration | wall time of the run |
| model | the model name the service reported (for example `typesafe/jev-1.13-20260917`) |

Exit status: run-level error or `failed > 0` → 2; else `kept > 0` → 0; else 1. A broken pipe ends
the run with 0.

# Data Model: Map Command

jevpipe keeps no stored data. A run flows through these in-memory types:
input → `Record` (or an input failure) → `Decision` → output line, with every decision counted into
the `Summary`. Changes against the first milestone are marked.

## Questions *(new)*

The questions sent with every request of a run.

| Field | Meaning |
|---|---|
| raw | the JSON object sent as the request's `questions`, byte for byte |
| asked | name → type (`noul`, `choice`, `score`) for each question; used to check the answers |

Built from:

- `map`: the questions file, after the shape check (research.md): a non-empty object; every question
  an object with a known `type` and non-null `instructions`; `choice.criteria` an object with 1 to 255
  entries; `score.criteria` an array with 2 to 10 entries; `noul.criteria`, when present, an object.
  Any failure is a usage error naming the file and, where one is at fault, the question.
- `filter`: `{"match": {"type": "noul", "instructions": <QUESTION>}}`.

## Input *(changed)*

What the reader hands the pipeline, in input order. Exactly one of:

| Variant | Carries | Becomes |
|---|---|---|
| record | line number, raw bytes, text | a request, or a skipped/failed decision |
| failed record | line number, raw bytes, text, reason (`not text`) | a failed decision |
| failed input *(new)* | input name, reason (`not found`, `UTF-16, convert it to UTF-8`, …) | a stderr line and a failed count; no output line |

- **line number** *(replaces position)*: 1-based, counts every line read (blank ones too), continuing
  across inputs in order. Blank or whitespace-only lines are counted but produce nothing.

## State (sent to the service) *(unchanged)*

| Mode | State |
|---|---|
| default | the line as a string (JSONL lines too) |
| `--read-files` | `{"path": <path>, "content": <text, at most 100,000 characters>}` |

## Decision *(changed)*

The outcome for one record:

| Outcome | Carries | Cause |
|---|---|---|
| answered | the answers (raw, as returned), truncated flag, cost, model | the service replied |
| skipped | reason: `directory`, `empty`, `binary` | `--read-files` skip rules |
| failed | reason: `not found`, other read errors, `not text`, `too large`, `service unavailable` | the record could not be decided |

`filter` derives kept or dropped from an answered decision: kept when `answers.match.noul` ≥ the
threshold. The first milestone's kept/dropped outcomes and their JSON form are gone.

## Output lines

`filter` stdout: the raw bytes of each kept record (unchanged).

`map` stdout, one per record, in input order:

```json
{"record":"src/net.rs","answers":{"relevant":{"type":"noul","noul":0.91}},"truncated":true}
{"record":"assets/logo.png","outcome":"skipped","reason":"binary"}
{"record":"src/gone.rs","outcome":"failed","reason":"not found"}
```

`truncated` appears only when true.

Standard error (both commands): `jevpipe: line <N>: <reason>` per failed record,
`jevpipe: <input name>: <reason>` per failed input, then the summary.

## Summary *(changed)*

| Field | Meaning |
|---|---|
| records | records and failed inputs seen |
| results | `filter`: kept; `map`: answered |
| skipped, failed | counts |
| truncated *(new)* | answered records whose file content was cut; printed only when > 0 |
| cost, duration, model | as before |

Exit status:

| Command | 0 | 1 | 2 |
|---|---|---|---|
| `filter` | something kept, nothing failed; or the output consumer went away | nothing kept, nothing failed | anything failed, or a run-level or usage error |
| `map` | nothing failed (empty input included); or the output consumer went away | — | anything failed, or a run-level or usage error |

# Research: Map Command

The first milestone's choices (runtime, HTTP client, ordered concurrency, retries, error classes,
record reading, file content, test stand-in) stay as recorded in
[../001-filter-foundation/research.md](../001-filter-foundation/research.md). This file covers what
`map` and the reshaped `filter` add or change.

## Question format (TypeSafe docs, checked 2026-09-27)

Sources: [API reference](https://docs.typesafe.ai/api), [Choice](https://docs.typesafe.ai/primitives/choice.md),
[Score](https://docs.typesafe.ai/primitives/score.md), [Noul](https://docs.typesafe.ai/primitives/noul.md).

| Field | Rule from the docs |
|---|---|
| `questions` | object; keys are chosen by the caller, no naming rules, no documented maximum count |
| every question | `type` (`noul`, `choice`, `score`) and `instructions` (string, object or array), both required |
| `noul.criteria` | optional; object with `true` and `false` descriptions |
| `choice.criteria` | required; object mapping option names to a description (string, object, array or null); at most 255 options |
| `score.criteria` | required; ordered array of level descriptions, low to high; 2 to 10 levels |
| answers | same keys as the questions; each carries `type`; noul: `noul`; choice: `choice`, `probabilities`, `confidence`; score: `score`, `legend`, `probabilities`, `confidence` |

The live spike ([../001-filter-foundation/api-spike.md](../001-filter-foundation/api-spike.md))
recorded a request with all three types in one call through OpenRouter, and the response matches this
shape. OpenRouter adds `usage.cost`.

## Questions file: validate, then send the bytes unchanged

- **Decision**: read the file once and keep it as `serde_json::value::RawValue` (feature `raw_value`)
  for the request, and parse it a second time into small typed structs for the shape check (FR-006).
  The check covers exactly the rules in the table above: non-empty object, known `type`, `instructions`
  present and not null, `criteria` present with the right kind and within the limits for choice and
  score, an object when given for noul. Other fields pass through untouched.
- **Rationale**: FR-005 says the file is sent unchanged; a raw value is unchanged byte for byte,
  including key order, and future question fields reach the service without a jevpipe change. The
  typed parse gives precise messages ("question `kind`: a choice needs 1 to 255 options").
- **Alternatives considered**: `serde_json::Value` with `preserve_order` (adds `indexmap` and
  re-serialises the file); a JSON Schema crate (heavy for six rules).

## Answers: check against the questions, pass through as returned

- **Decision**: take `answers` from the response as a `RawValue` and write it to the output as is.
  Before that, parse it into `name → {type}` and check that every asked question has an answer of the
  asked type; if not, the run stops with "unexpected answer" (as the first milestone does for a noul
  outside 0..=1). `filter` reads its probability from the same answers.
- **Rationale**: FR-011 wants the answers exactly as returned; the check keeps a changed service from
  silently producing output a script would misread.

## One engine, two thin commands

- **Decision**: one pipeline module (`pipeline.rs`) runs records → state → one request with the run's
  questions → outcome, with ordered concurrency, and feeds every decision into a summary and a
  command-specific output. `filter` and `map` differ only in the questions they send (a single noul
  named `match` built from the question string, or the questions file), in what they print and in the
  exit rule. The output side is a small trait with two implementations; the rest is shared code.
- **Rationale**: Constitution III; the first milestone already split reading, service and output. The
  service client now takes the questions instead of a fixed noul, so both commands use the same
  request path.
- **Alternatives considered**: an enum of the two modes matched in the pipeline: mixes both commands'
  output rules into one module.

## Line numbers and input failures

- **Decision**: the reader counts every line it reads, blank or not, continuing across inputs, and
  stamps each record with that line number. An input that cannot be opened, cannot be read, or starts
  with a UTF-16 byte-order mark is sent to the pipeline as an input failure carrying the input's name,
  not as a record: it is reported by name, counted as failed, and has no output line.
- **Rationale**: FR-009, FR-014; `sed -n <line>p` over the joined input finds the record.

## CLI shape

- **Decision**: `filter` declares `<QUESTION>`, then `[FILES]...`. `map` takes its questions from
  `-q/--questions <JSON>` or `-f/--questions-file <FILE>`, a clap argument group with
  `required = true, multiple = false`, and all its positionals are `[FILES]...`. Both flatten one
  shared `#[derive(Args)]` struct with `--read-files`, `--concurrency`, `--model` and
  `--request-timeout`. `map`'s `after_help` shows the questions with all three types.
- **Why two options for `map`**: inline questions suit agents (one command, no file to write or
  clean up), a file suits people and long question sets. A grep-style positional with `-f` would
  need hand-rolled parsing: clap cannot turn the first positional into a file when `-f` is given
  (a prototype assigned it to the questions). `@file` (curl style) collides with PowerShell's
  splatting operator. Sniffing a leading `{` gives one argument two meanings. clap's derive does
  not support an enum for the two options, so they are two `Option` fields in a required group.
- **Rationale**: positionals stay in their natural order per command without relying on the order of
  flattened fields; the shared flags are defined once.

## Output

- **Decision**: `filter` writes the kept records' raw bytes (unchanged from the first milestone).
  `map` writes one `serde_json` line per decision with the record as a string (lossy UTF-8 for a line
  that is not valid UTF-8) and flushes after each. `truncated` is serialised only when true.
  Standard error: `jevpipe: line N: <reason>` or `jevpipe: <input name>: <reason>`, then the summary,
  which adds `, N truncated` when N > 0 and names the result count `kept` or `answered`.
- **Rationale**: flushing per line is what makes the step loop work (SC-004); failures on standard
  error never carry record content (FR-014).

## Test stand-in

- **Decision**: the stand-in answers every question in the request by its type, using markers in the
  record as today: `p=` for nouls, `choice=<option>` (default: the first option) and `level=<n>`
  (default: 0) for scores, with answer bodies shaped like the spike's recorded response. The test
  crate `tests/filter/` becomes `tests/pipeline/`, shared by the `filter` and `map` tests.
- **Rationale**: FR-018; one stand-in for both commands keeps the helpers in one crate, so none count
  as dead code.

## Dependencies

- **Decision**: enable `serde_json`'s `raw_value` feature; add `encoding_rs` 0.8.42 for decoding file
  content (below). No other new crates; `tokio-stream`'s `ReceiverStream` would replace three lines in
  `record.rs` and is not worth a dependency.

## File decoding

- **Decision**: replace `text.rs`'s hand-rolled UTF-8/UTF-16 decoding with `encoding_rs`: a decoder
  with BOM sniffing (UTF-8 by default, UTF-16 LE/BE by byte-order mark) and
  `decode_to_string_without_replacement`, with `last` false when the read stopped at the byte limit, so
  a character cut by the limit stays undecoded instead of failing. Malformed input means binary, as
  before; the NUL check runs on the decoded text. The line reader's UTF-16 check uses
  `Encoding::for_bom`.
- **Rationale**: the project prefers a proven crate over hand-rolled code; `encoding_rs` is Firefox's
  decoder and what ripgrep uses (through `encoding_rs_io`). It removes the surrogate and partial-read
  handling, about 45 lines. Its licenses (Apache-2.0 OR MIT, BSD-3-Clause for the data) are already
  allowed in `deny.toml`.
- **To verify at implementation**: the behaviour of the first milestone's file tests (UTF-16 files,
  cut characters, BOM-only files) is unchanged.

# Implementation Plan: Map Command

**Branch**: `002-map-command` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/002-map-command/spec.md`

## Summary

Add `jevpipe map (-q <JSON> | -f <FILE>) [FILES]...`: the questions (System One `questions`, inline
or from a file, checked, then sent byte for byte) is asked about every record in one request, and each record gets one JSON
line `{"record", "answers"}` (or `outcome`/`reason`) in input order. `filter` loses `--json`/`--all`
and becomes pure grep. Both commands share one pipeline: the service client takes the run's questions
instead of a fixed noul, `filter` sending a single noul named `match`. Records carry line numbers
(blank lines counted), failures on standard error name the line, never the record, and input files
that cannot be read become input failures reported by name.

## Technical Context

**Language/Version**: Rust 1.98 (edition 2024), pinned in `rust-toolchain.toml`
**Primary Dependencies**: unchanged from the first milestone (`clap`, `tokio`, `reqwest`, `futures`,
`backon`, `serde`, `serde_json`, `thiserror`); `serde_json` gains the `raw_value` feature. No new crates.
**Storage**: N/A (stateless)
**Testing**: `cargo test`; integration tests run the real binary against the `wiremock` stand-in,
extended to answer choice and score questions; no network, no API key
**Target Platform**: Linux, Windows, macOS (CI matrix); single binary
**Project Type**: single CLI crate (library + thin binary)
**Performance Goals**: 1,000 records with three questions in under 30 s (SC-003); in a step loop each
answer written within 1 s of the service's reply (SC-004)
**Constraints**: bounded memory on unbounded input; questions and answers passed through unchanged;
no record content on standard error; stdout written only through one writer
**Scale/Scope**: two subcommands, ~12 modules, ~1,300 lines

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass: one new command, no `serve`, no per-record questions, no structured state; `filter` loses two options |
| II. Automated Verification | Pass: every user story gets offline integration tests against the real binary; `check.ps1` gates format, clippy, tests and `cargo deny` |
| III. Reusable Components | Pass: both commands run on one pipeline, one service path and one reader; only output and exit rules are per command |

Post-design re-check: unchanged. The design adds one module per new concept (questions, pipeline,
map output) and no persistent state.

## Project Structure

### Documentation (this feature)

```text
specs/002-map-command/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md           # both commands: options, questions file, stdout/stderr, exit status
│   └── service.md       # questions and answers on the wire
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── main.rs        # unchanged
├── lib.rs         # modules, `run` dispatching filter and map
├── cli.rs         # filter and map; shared flags in one flattened Args struct
├── questions.rs   # NEW: load + shape-check the questions file (a clap value parser); filter's single noul
├── answers.rs     # NEW: answers as returned + the check against the questions; PROBABILITY
├── reason.rs      # NEW: Skip and Failure reasons (from decision.rs), shared by reader, file and pipeline
├── record.rs      # CHANGED: own Input enum, line numbers (blank lines counted), text + line ending
├── file.rs        # CHANGED: returns its own skip/fail reason instead of an Outcome
├── text.rs        # CHANGED: decoding via encoding_rs; only the NUL check is left
├── service.rs     # CHANGED: ask(questions, state) -> raw answers + cost + model
├── decision.rs    # CHANGED: Answered / Skipped / Failed; no serde
├── pipeline.rs    # NEW: records → buffered(concurrency) decisions → output + summary (from filter.rs)
├── filter.rs      # CHANGED: thin: Command impl; builds the noul, threshold + PROBABILITY, raw output
├── map.rs         # NEW: thin: Command impl; its own output line struct
├── output.rs      # CHANGED: stdout writer + broken pipe, stderr failure lines (line / input name)
└── summary.rs     # CHANGED: result label, truncated count

tests/
├── cli.rs              # usage errors (incl. filter --json/--all, bad questions files), help texts
└── pipeline/           # renamed from tests/filter/: one crate, shared stand-in
    ├── main.rs
    ├── stand_in.rs     # answers noul, choice and score questions from markers in the record
    ├── fixture.rs
    ├── filter_lines.rs # was lines.rs
    ├── filter_files.rs # was files.rs
    ├── streaming.rs    # both commands: order, broken pipe, timeouts, step loop
    ├── map_answers.rs  # user story 1
    ├── map_files.rs    # user story 2
    └── stderr.rs       # user story 3: line numbers, no record content, input failures
```

`tests/filter/json.rs` is deleted; what it covered moves to `map_answers.rs` and `map_files.rs`.

**Structure Decision**: single crate, flat `src/` (16 modules of about 100 lines, one concept each;
subfolders pay off at around 20 modules). The old `filter.rs` pipeline moves to `pipeline.rs` so both
commands are thin modules over it.

### Other repository changes

- `Cargo.toml`: `serde_json` feature `raw_value`; add `encoding_rs` (licenses already allowed in
  `deny.toml`).
- `specs/manual/idea-draft.md`: `serve` dropped, `map` and the step loop as decided; "Later" list
  updated.
- `CLAUDE.md` (at implementation, not by this plan): planned shape `filter` and `map`, no `serve`.
- `tests/cli.rs`: `--json` and `--all` leave the help and usage lists; `filter --json` becomes an
  unknown-option case.
- `specs/001-filter-foundation/` stays as the record of the first milestone and is not edited.

## Code Structure

Refactoring the first milestone's code alongside `map`, so the new command does not deepen existing
couplings:

1. **Domain types carry no output format.** `Outcome` and `Judgment` lose `Serialize` and their
   `#[serde(skip)]` fields; `map.rs` owns its output line struct (`record`, `answers`, `truncated` /
   `outcome`, `reason`).
2. **The reader produces input, not decisions.** `record.rs` returns its own enum (record, line that
   is not text, failed input with its name) instead of `Result<Record, Decision>`; the pipeline turns
   it into decisions.
3. **Threshold logic lives in `filter.rs`.** `Outcome::judged` and kept/dropped move there; the
   shared outcome is answered / skipped / failed. `PROBABILITY` lives in `answers.rs`, which checks
   every noul against it; `cli.rs` takes it from there for `--threshold`.
4. **One trait for what differs per command.** `pipeline.rs` is generic over a small `Command` trait
   (write a decision, whether it counts as a result, the result label, the exit rule), implemented by
   `filter` and `map`; static dispatch, no `dyn`. `Exit::NothingKept` stops being a special case in
   `summary.rs`.
5. **A line is stored once.** `Record` keeps the text and its line ending (LF, CRLF or none)
   instead of raw bytes plus text; `filter` writes text + ending, byte-identical to the input. A line
   that is not text keeps its lossy text for `map`'s `record`. Drop this if it makes the reader
   awkward.
6. **Shared test helpers.** `json_lines` and the stand-in's choice and score answers live in the
   shared files of `tests/pipeline/`.
7. **The service client knows no command.** `service.rs`'s fixed `Questions` / `Question::Noul` /
   `Answers` / `Noul` types go; it sends the run's questions and returns the raw answers, cost and
   model. `filter` reads `answers.match.noul` itself.
8. **File reading returns its own reason.** `file::read` returns `Result<Content, Unjudged>` (skip or
   fail reason) instead of `Result<Content, Outcome>`.
9. **Removed with `--json`:** `output::Format` and `Format::new`, `JsonLine` and `position`, the
   hand-written `impl Serialize for Failure` (the reason is written as text), and the
   `Outcome::skipped` / `failed` / `unreadable` wrappers.
10. **`encoding_rs` instead of hand-rolled decoding.** `text::decode` and its UTF-16 / surrogate /
    partial-read code are replaced by `encoding_rs`'s decoder with BOM sniffing and
    `decode_to_string_without_replacement(…, last = !partial)`, which leaves a character cut by the
    read limit undecoded instead of failing. `text.rs` keeps only the NUL check, applied to the
    decoded text; the line reader's UTF-16 check uses `Encoding::for_bom`.

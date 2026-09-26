---
description: "Task list for the filter foundation"
---

# Tasks: Filter Foundation

**Input**: Design documents from `specs/001-filter-foundation/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/cli.md](contracts/cli.md),
[contracts/service.md](contracts/service.md), [api-spike.md](api-spike.md)

**Tests**: Required by FR-023. Integration tests run the real binary (`assert_cmd`) against the
`wiremock` stand-in; no network, no API key. Write each story's tests first and see them fail.

**Rules for every task**: follow `CLAUDE.md` (no comments, max 3 parameters besides `self`,
exhaustive `match` without `_` on enums, no `unwrap`/`expect`/`panic!` in `src/`, private by
default). clap help text goes in `#[arg(help = "...")]` / `#[command(about = "...")]` attributes,
not doc comments. `./check.ps1 -Fix` passes at the end of each phase.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1, US2, US3)

---

## Phase 1: Setup

**Purpose**: dependencies and the module skeleton from plan.md.

- [ ] T001 Add dependencies to `Cargo.toml` with `cargo add`: `tokio` (features `rt-multi-thread`, `macros`, `io-std`, `io-util`, `fs`), `reqwest` (feature `json`, default features kept), `futures`, `backon`, `serde` (feature `derive`), `serde_json`, `thiserror`; dev: `wiremock`, `tempfile`, and `tokio` with the same features
- [ ] T002 [P] Extend `licenses.allow` in `deny.toml` with `ISC`, `BSD-3-Clause`, `MIT-0`, `CDLA-Permissive-2.0` (research.md, Licenses)
- [ ] T003 Create empty modules `src/cli.rs`, `src/record.rs`, `src/file.rs`, `src/service.rs`, `src/decision.rs`, `src/filter.rs`, `src/output.rs`, `src/summary.rs`, declare them in `src/lib.rs`, and change `src/main.rs` to a `#[tokio::main]` that parses the CLI, awaits `jevpipe::run` and returns its `std::process::ExitCode`

**Checkpoint**: `./check.ps1 -Fix` passes; `jevpipe --version` still works.

---

## Phase 2: Foundational

**Purpose**: the stand-in, the CLI surface, the service client and the shared result types every story
uses. **No story work before this phase is done.**

- [ ] T004 [P] Create the integration test crate `tests/filter/main.rs` declaring `mod stand_in;` and one module per story file below (one crate, so every shared helper is used and no dead-code warning fails the lint stage). Build the stand-in in `tests/filter/stand_in.rs`: a `wiremock::MockServer` on `POST /v1/systemone` with a custom `Respond` that reads the request body (shape in contracts/service.md) and answers per record from markers in the state text (the string state, or the `content` field of an object state): `p=0.93` sets `answers.match.noul` (default `0.1`); `fail=503x2` answers `503` with `Retry-After: 0` for the first 2 attempts of that record, then normally (also `429`, `524`); `fatal=401` always answers that status (also `400`, `402`); `toolarge` answers `400` with the `max_tokens_exceeded` body; `slow=1500` delays the first attempt by that many milliseconds. Response and error bodies copy the shapes in api-spike.md, with `usage.cost` `0.00001` and model `typesafe/jev-test`. Attempt counts per record are kept in a `Mutex<HashMap<String, usize>>`. Provide `StandIn::start()`, `StandIn::requests()` (received request count), and `StandIn::jevpipe()` returning an `assert_cmd::Command` for the real binary with `JEVPIPE_BASE_URL` set to the server and `OPENROUTER_API_KEY=test-key`
- [ ] T005 [P] Define the CLI in `src/cli.rs` per contracts/cli.md: subcommand `filter` with positional `question` (non-empty) and `files`, `--read-files`, `--threshold` (f64, 0..=1, default 0.5), `--json`, `--all` (`requires = "json"`), `--concurrency` (≥ 1, default 100), `--model` (default `~typesafe/jev-latest`), `--request-timeout` (seconds ≥ 1, default 10); help text on every argument; usage errors exit with status 2 (clap's default)
- [ ] T006 [P] Implement `src/decision.rs` per data-model.md: `Outcome` enum `Kept { probability }`, `Dropped { probability }`, `Skipped { reason }`, `Failed { reason }`; `Decision { position, record: String, raw: Vec<u8>, outcome, truncated: bool }`; a constructor that turns a probability and threshold into `Kept` or `Dropped` (kept when probability ≥ threshold)
- [ ] T007 [P] Implement `src/summary.rs` per data-model.md: counts (records, kept, skipped, failed), summed optional cost, last reported model, start time; `add(&Decision)`; the exit status (failed > 0 → 2, kept > 0 → 0, else 1); rendering the summary line exactly as in contracts/cli.md (cost omitted when none reported, model omitted when no request was made)
- [ ] T008 Implement `src/service.rs` per contracts/service.md and research.md: `ServiceConfig::from_env()` reading `OPENROUTER_API_KEY` (error naming the variable when missing or empty) and `JEVPIPE_BASE_URL` (default `https://openrouter.ai/api`); a `Service` holding one `reqwest::Client` (timeout from `--request-timeout`) and the model; `ask(&self, question: &str, state: serde_json::Value) -> Result<Answer, ServiceError>` posting one `noul` question named `match` without criteria; `Answer { probability, cost: Option<f64>, model }`; `ServiceError` (`thiserror`) with `Transient`, `TooLarge` and `Run { message }` classified by the status table; retries with `backon` exponential backoff with jitter (0.5 s start, 8 s cap, 4 retries) only `.when` transient, `.adjust`ed to the `Retry-After` seconds when the response has one

**Checkpoint**: foundation compiles, `./check.ps1 -Fix` passes.

---

## Phase 3: User Story 1 - Filter lines by a yes/no question (Priority: P1) 🎯 MVP

**Goal**: `jevpipe filter "<question>" [FILE]...` keeps the lines answered "yes", unchanged, in order,
streaming, with retries, summary and exit status.

**Independent Test**: lines with `p=` markers through `filter` against the stand-in; the output is
exactly the lines at or above the threshold, byte-identical and in order.

### Tests for User Story 1

- [ ] T009 [P] [US1] Write `tests/filter/lines.rs`, one test per behaviour: keeps lines at or above 0.5 byte-identical and in input order (US1-1); reads files in argument order and `-` as stdin (US1-2, FR-001); `--threshold 0.8` drops a 0.79 line (US1-3); exit 0 plus exactly one summary line when something is kept, exit 1 when nothing is (US1-4, US1-5); transient `503`/`429`/`524` for two attempts still yields the same output and exit 0 (US1-6, SC-004); exhausted retries report `jevpipe: record N (...)` on stderr, keep the other records and exit 2 (US1-7); JSONL and `\r\n` lines come out byte-identical (US1-8, edge cases); blank lines are not records and not counted; empty input exits 1 with zero requests; `fatal=401` stops the run with `jevpipe: error:` and exit 2; missing `OPENROUTER_API_KEY` exits 2 naming the variable with zero requests; a named input file that does not exist is reported as a failed record at its place while the other files are still judged, exit 2
- [ ] T010 [P] [US1] Add to `tests/cli.rs` (no stand-in needed): empty question, threshold `1.5` and `--concurrency 0` are usage errors with exit 2; `filter --help` lists every option in contracts/cli.md (FR-022)
- [ ] T011 [P] [US1] Write `tests/filter/streaming.rs`: with stdin held open after one `p=0.9` line, the first stdout line arrives within 2 s (SC-003); with the reader closing after the first line (`std::process::Child`, drop stdout), jevpipe exits 0, its stderr holds only the summary line, and the stand-in receives far fewer requests than there are records (US1-9, FR-018); 200 records with `slow=300` finish in under 10 s at the default concurrency (SC-002); a `slow=1500` record with `--request-timeout 1` is retried and kept (FR-011)

### Implementation for User Story 1

- [ ] T012 [US1] Implement `src/record.rs`: an async stream of records over the inputs in order (files opened with `tokio::fs`, `-` and no files → stdin), reading with `read_until(b'\n')`, skipping empty or whitespace-only lines, numbering records from 1 across all inputs, keeping the raw bytes including the terminator, and exposing the text without its terminator or a `not UTF-8` failure; each named file is opened when the stream reaches it; one that cannot be opened yields one failed record at its place (the file name as the record, the reason such as `not found`) and reading continues with the next file
- [ ] T013 [US1] Implement raw output in `src/output.rs`: one writer over a locked `std::io::stdout()` that writes a kept record's raw bytes (adding `\n` when the last line has none) and flushes per record; a `BrokenPipe` write error becomes a distinct `Closed` result; failed records write `jevpipe: record {position} ({record}): {reason}` to stderr
- [ ] T014 [US1] Implement the pipeline in `src/filter.rs`: map each record to a future that returns its `Decision` (text state for lines; `ServiceError::Transient` after retries → failed `service unavailable`, `TooLarge` → failed `too large`, `Run` → stop), run them with `futures::StreamExt::buffered(concurrency)`, feed each decision to the summary and the writer, stop on `Closed` (summary still written, exit 0) or a run error (message `jevpipe: error: {message}`, exit 2)
- [ ] T015 [US1] Implement `run` in `src/lib.rs`: read `ServiceConfig` before any input, build `Service`, run `filter`, write the summary line to stderr, return the exit status as `ExitCode`

**Checkpoint**: US1 tests pass; `./check.ps1 -Fix` passes. MVP usable with plain lines.

---

## Phase 4: User Story 2 - Semantic grep over files (Priority: P2)

**Goal**: `--read-files` judges each listed file's path and content and emits matching paths.

**Independent Test**: temp text files with `p=` markers, a binary, an empty file, a directory and a
missing path through `filter --read-files`; matching paths out, skips and failures counted.

### Tests for User Story 2

- [ ] T016 [P] [US2] Write `tests/filter/files.rs` with files in a `tempfile::TempDir` per test: matching paths printed unchanged in input order and the stand-in received `{"path","content"}` state (US2-1); a file with a NUL byte and a non-UTF-8 file are skipped with zero requests for them and exit status unaffected (US2-2); an empty file and a directory are skipped (US2-3); a missing path is reported on stderr, counted failed, exit 2 (US2-4); a 150,000-character file is sent with at most 100,000 characters and still kept (US2-5, FR-006); a `toolarge` file fails with reason `too large`

### Implementation for User Story 2

- [ ] T017 [US2] Implement `src/file.rs` per research.md: from a path, return content, skip or failure: directory → skipped `directory`; open or read error → failed with the error kind (`not found`, `permission denied`, …); read at most 400,000 bytes; empty → skipped `empty`; NUL byte in the first 8 KiB → skipped `binary`; invalid UTF-8 (except a character cut by the read limit at the very end) → skipped `binary`; truncate to 100,000 characters on a character boundary and report `truncated`; the state is `{"path": <path>, "content": <text>}`
- [ ] T018 [US2] Use `file.rs` in `src/filter.rs` when `--read-files` is set: skipped and failed files produce their decision without a service call, the emitted record is the path line's raw bytes, and `truncated` is carried into the decision

**Checkpoint**: US1 and US2 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 5: User Story 3 - Inspect the decisions (Priority: P3)

**Goal**: `--json` writes kept records as JSON objects; `--json --all` writes every record with its
outcome.

**Independent Test**: one kept, one dropped, one skipped record: `--json` prints one object, `--json --all`
three, all parseable.

### Tests for User Story 3

- [ ] T019 [P] [US3] Write `tests/filter/json.rs`: `--json` prints one object per kept record with `position`, `record`, `outcome: "kept"`, `probability`, `truncated`, in the same order as the raw output (US3-1, US3-2); `--json --all` prints one object per record including `dropped` with `probability` and `skipped`/`failed` with `reason` (US3-3); a truncated file shows `"truncated": true`; `--all` without `--json` exits 2 (US3-4); exit status and summary match the raw run (US3-5); every stdout line parses with `serde_json`

### Implementation for User Story 3

- [ ] T020 [US3] Add JSON output to `src/output.rs`: a `serde::Serialize` line type matching the objects in contracts/cli.md (`record` as a string without its terminator; `probability` and `truncated` on kept/dropped, `reason` on skipped/failed); the writer's mode is raw, JSON (kept only) or all (every decision), chosen from `--json` and `--all`

**Checkpoint**: all stories pass; `./check.ps1 -Fix` passes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T021 Review `jevpipe filter --help` against SC-007: a `grep` user can run a first semantic search from it alone; tighten help texts in `src/cli.rs`
- [ ] T022 Run every command in quickstart.md against the real service with `OPENROUTER_API_KEY` and note surprises in `specs/001-filter-foundation/quickstart.md`
- [ ] T023 Run `./check.ps1` (strict, `--locked`) and push the branch; CI passes on Linux, Windows and macOS (SC-006)

---

## Dependencies & Execution Order

- **Setup (T001–T003)** → **Foundational (T004–T008)** → **US1 (T009–T015)** → **US2 (T016–T018)** and
  **US3 (T019–T020)** → **Polish (T021–T023)**.
- T001 before everything that compiles; T003 before T005–T008.
- T008 depends on T005 (timeout, model) only for wiring; it can be written in parallel.
- US2 and US3 both extend `src/filter.rs` / `src/output.rs` from US1, so they start after US1; they are
  independent of each other (US3's truncated test needs US2 for `--read-files`; run it last or skip that
  one assertion until US2 is in).

### Parallel Opportunities

- Phase 1: T002 alongside T001/T003.
- Phase 2: T004, T005, T006, T007 in parallel; then T008.
- US1: T009, T010, T011 (three test files) in parallel; then T012 → T013 → T014 → T015.
- US2 tests (T016) and US3 tests (T019) can be written in parallel once US1 is done.

## Implementation Strategy

1. **MVP**: Phases 1–3. Stop and verify: `printf 'a\nb\n' | jevpipe filter "..."` works against the
   real service, all US1 tests pass.
2. **Increment**: US2 (`--read-files`), the flagship use case; then US3 (`--json`, `--all`).
3. **Finish**: Polish, push, CI green on all three platforms; stop for review before `map`/`serve`
   (idea-draft.md).

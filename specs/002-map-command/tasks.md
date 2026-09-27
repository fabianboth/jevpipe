---
description: "Task list for the map command"
---

# Tasks: Map Command

**Input**: Design documents from `specs/002-map-command/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/cli.md](contracts/cli.md),
[contracts/service.md](contracts/service.md), [quickstart.md](quickstart.md)

**Tests**: Required by FR-018. Integration tests run the real binary (`assert_cmd`) against the
`wiremock` stand-in; no network, no API key. Write each story's tests first and see them fail.

**Rules for every task**: follow `CLAUDE.md` (no comments, max 3 parameters besides `self`,
exhaustive `match` without `_` on enums, no `unwrap`/`expect`/`panic!` in `src/`, private by
default, one module per concept). clap help text goes in doc comments on the clap types.
`./check.ps1 -Fix` passes at the end of each phase. The numbered points below refer to plan.md,
"Code Structure".

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1, US2, US3)

---

## Phase 1: Setup

**Purpose**: dependencies and the renamed test crate.

- [X] T001 Enable the `raw_value` feature of `serde_json` and add `encoding_rs` with `cargo add` in `Cargo.toml`; run `cargo deny check` and confirm its licenses pass the existing `deny.toml` (research.md, Dependencies)
- [X] T002 [P] Rename the test crate with `git mv`: `tests/filter/` → `tests/pipeline/`, `lines.rs` → `filter_lines.rs`, `files.rs` → `filter_files.rs`; update the `mod` lines in `tests/pipeline/main.rs`; move the `json_lines` helper from `json.rs` into `tests/pipeline/fixture.rs` (point 6)

**Checkpoint**: `./check.ps1 -Fix` passes with the unchanged product.

---

## Phase 2: Foundational

**Purpose**: the shared engine both commands run on (plan.md, Code Structure 1–10). `filter` keeps its
behaviour except for what the spec changes: `--json`/`--all` are removed, and standard error names
line numbers and input names instead of records. **No story work before this phase is done.**

- [X] T003 [P] Replace the decoding in `src/text.rs` with `encoding_rs` (point 10, research.md "File decoding"): BOM-sniffing decoder, UTF-8 by default, `decode_to_string_without_replacement` with `last = !partial`, malformed → `None` (binary); the NUL check runs on the decoded text; keep `is_utf16` as a thin wrapper over `Encoding::for_bom` or replace its caller's use with it. The existing `tests/pipeline/filter_files.rs` tests for UTF-16, broken UTF-16, BOM and cut characters must still pass unchanged
- [X] T004 [P] Change `src/file.rs` to return `Result<Content, Unjudged>`, where `Unjudged` is its own enum of a skip reason or a failure reason, instead of `Result<Content, Outcome>` (point 8)
- [X] T005 [P] Rewrite `src/decision.rs` per data-model.md (points 1, 3, 9): `Outcome` is `Answered(Reply)` with the truncated flag, `Skipped { reason: Skip }` or `Failed { reason: Failure }`; no `Serialize` derive or impl anywhere in the module; `Failure` and `Skip` implement `Display` (the reason texts stay as in the first milestone); remove `Kept`, `Dropped`, `Judgment`, `Outcome::judged`, `skipped`, `failed`, `unreadable` and `PROBABILITY`
- [X] T006 [P] Create `src/questions.rs` with the run's questions (data-model.md, Questions): `Questions { raw: Box<RawValue>, asked }` where `asked` maps each question name to its type (`Noul`, `Choice`, `Score`); `Questions::yes_no(question: &str)` builds `{"match": {"type": "noul", "instructions": …}}`; a check that a response's `answers` holds every asked name with an answer of the asked `type`, returning an error message for "unexpected answer: …" otherwise (research.md, Answers). Loading a questions file comes in US1
- [X] T007 Rework `src/service.rs` (point 7, contracts/service.md): `ask(&self, questions: &Questions, state: &State<'_>)` sends `questions.raw` unchanged as the request's `questions`; returns `Reply { answers: Box<RawValue>, cost: Option<f64>, model: String }`; the answers check from T006 runs on every `200` and a failure is `ServiceError::Rejected("unexpected answer: …")`; remove `Questions`, `Question`, `Answers`, `Noul` and `Answer`; retries, timeout and error classes stay as they are
- [X] T008 Rework `src/record.rs` (points 2, 5, research.md "Line numbers and input failures"): the reader sends its own enum: a record `{ line, text, ending }` (ending LF, CRLF or none), a line that is not text `{ line, text }` (lossy), or a failed input `{ name, failure }` for an input that cannot be opened or read or starts with a UTF-16 BOM (via `encoding_rs::Encoding::for_bom`); line numbers count every line read, blank and whitespace-only ones included, continuing across inputs; blank lines produce nothing; `record.rs` no longer imports `decision`
- [X] T009 [P] Rework `src/output.rs` (point 9): one stdout writer with `write_line(bytes)`-style methods that flush per call and report `BrokenPipe` as `Delivery::Closed`; stderr lines `jevpipe: line {N}: {reason}` for a failed record and `jevpipe: {name}: {reason}` for a failed input, never record content; remove `Format`, `Format::new` and `JsonLine`
- [X] T010 [P] Rework `src/summary.rs`: count records (failed inputs included), results, skipped and failed; the result label (`kept` or `answered`) comes from the command; `Exit` stays `Success` / `NothingKept` / `Error`, but which one applies is decided by the command's exit rule (T011), not by `summary.rs`
- [X] T011 Create `src/pipeline.rs` from the pipeline half of `src/filter.rs` (point 4, research.md "One engine"): a `Command` trait with the decision writer, whether a decision counts as a result, the result label and the exit rule; `run` generic over it (static dispatch) that takes the shared flags and `Questions`, reads `ServiceConfig` before any input, builds the state per record (text, or `{path, content}` with `--read-files` via `file::read`), maps reader input and `Unjudged` to decisions, runs `buffered(concurrency)`, adds each decision to the summary, reports failures on stderr, stops on `Delivery::Closed` or a run error, and prints the summary line; `RunError` moves here
- [X] T012 Reduce `src/filter.rs` to the `Command` implementation (point 3): `Questions::yes_no`, reads `answers.match.noul` from the reply and stops the run with "unexpected answer: probability … is not between 0 and 1" outside `PROBABILITY` (defined here), kept when ≥ `--threshold`, writes the record's text plus its ending (adding LF when the last line has none), counts kept records, exit rule as in contracts/cli.md
- [X] T013 Rework `src/cli.rs` and `src/lib.rs`: one `#[derive(Args)]` struct with `--read-files`, `--concurrency`, `--model`, `--request-timeout` flattened into `filter` (positionals `question`, then `files`); remove `--json` and `--all`; `--threshold`'s parser uses `filter::PROBABILITY`; `lib.rs` declares `questions` and `pipeline` and dispatches `filter`
- [X] T014 Update the tests to the changed behaviour: delete `tests/pipeline/json.rs` and its `mod` line; in `tests/pipeline/filter_lines.rs` and `filter_files.rs` replace `jevpipe: record N (…): reason` expectations with `jevpipe: line N: reason`, and a missing named input file with `jevpipe: gone.txt: not found` (no line number); in `tests/cli.rs` remove `--json`/`--all` from the help list and the `--all` usage case, add `filter … --json` as an unknown-option case; extend `tests/pipeline/stand_in.rs` to answer every question in the request by its type (point 6, research.md "Test stand-in"): noul from `p=` (default 0.1), choice from `choice=<option>` (default the first option; `probabilities` 1 for it, 0 for the others; `confidence` 1), score from `level=<n>` (default 0; `score` n, `legend` from the criteria, `probabilities` 1 at n, `confidence` 1), body shapes as in the spike's recorded response; `malformed` still answers without the answers

**Checkpoint**: every `filter` test passes; `./check.ps1 -Fix` passes; `jevpipe filter --json` is an
unknown option.

---

## Phase 3: User Story 1 - Ask several typed questions about every record (Priority: P1) 🎯 MVP

**Goal**: `jevpipe map <QUESTIONS_FILE> [FILES]...` asks all questions about each record in one
request and writes one JSON line per record.

**Independent Test**: three lines through `map` with a noul, a choice and a score question; three
JSON lines in order with the stand-in's answers, one request per line carrying all three questions.

### Tests for User Story 1

- [X] T015 [P] [US1] Write `tests/pipeline/map_answers.rs`, one test per behaviour: three lines with a noul, a choice and a score question give three lines `{"record", "answers"}` in input order with exactly the stand-in's answers, and the stand-in received one request per line whose `questions` equal the file's content (US1-1, US1-2, SC-001, SC-002); a JSONL line is sent as a string state and comes back as a string `record` that parses as the original object (US1-3); two files in argument order (US1-4); exit 0 with `3 records, 3 answered, 0 skipped, 0 failed` (US1-5); a record failing after all retries gets `{"record", "outcome": "failed", "reason": "service unavailable"}`, the others are answered, stderr has `jevpipe: line N: service unavailable`, exit 2 (US1-6); empty input exits 0 with zero requests; `malformed` (no answers) stops the run with `unexpected answer`, exit 2; `status=401` stops the run with `jevpipe: error:`, exit 2
- [X] T016 [P] [US1] Add to `tests/cli.rs` (no stand-in needed): each malformed questions file is a usage error with exit 2 and a message naming the file (and the question where one is at fault): missing file, not JSON, not an object, empty object, unknown `type`, missing or null `instructions`, choice without `criteria` or with 0 or 256 options, score with 1 or 11 levels or non-array criteria, noul `criteria` that is not an object (FR-006, SC-006); `map --help` shows `Usage: jevpipe map [OPTIONS] <QUESTIONS_FILE> [FILES]...`, every option of contracts/cli.md for `map`, and a questions file example (FR-017)
- [X] T017 [P] [US1] Add to `tests/pipeline/streaming.rs`: with stdin held open, `map` writes the answer to a first line within 1 s, before the second line is written (US1-7, SC-004); `map` stops quietly when the reader goes away, exit 0; 200 records with three questions and `slow=300` finish in under 10 s at the default concurrency (SC-003)

### Implementation for User Story 1

- [X] T018 [US1] Add loading to `src/questions.rs` (research.md "Questions file"): read the file, keep it as `Box<RawValue>`, parse it a second time into typed structs and check the shape (non-empty object; known `type`; non-null `instructions`; choice `criteria` an object with 1 to 255 entries; score `criteria` an array with 2 to 10 entries; noul `criteria` an object when present); errors carry the file name and the question name, e.g. ``questions.json: question `kind`: a choice needs criteria with 1 to 255 options``
- [X] T019 [US1] Add the `map` subcommand to `src/cli.rs`: positional `questions_file` (`PathBuf`), then `files`, the shared flags flattened; long help per contracts/cli.md with the questions file example from quickstart.md in `after_help`
- [X] T020 [US1] Create `src/map.rs` with the `Command` implementation (point 1): its own serializable line structs `{ record, answers, truncated }` (`answers` written as the `RawValue`, `truncated` skipped when false) and `{ record, outcome, reason }`; `record` is the record's text (lossy for a line that is not text); every decision is written; result label `answered`; exit `Success` unless something failed; load the questions (T018) before `pipeline::run`, a load error exits 2 with `jevpipe: error: …` before any input is read
- [X] T021 [US1] Declare `map` in `src/lib.rs` and dispatch the subcommand

**Checkpoint**: US1 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 4: User Story 2 - Ask questions about files (Priority: P2)

**Goal**: `map --read-files` judges each file's path and content and echoes the path as the record.

**Independent Test**: text files and a binary file through `map --read-files`; answered lines with
the paths, a skipped line for the binary, exit 0.

- [X] T022 [P] [US2] Write `tests/pipeline/map_files.rs`: paths through `map --read-files` are answered with the path as `record` and the state `{path, content}` (US2-1); binary, non-UTF-8, empty files and directories get `{"record", "outcome": "skipped", "reason"}` without a request, exit 0 (US2-2); a missing path gets a failed line with `not found`, exit 2 (US2-3); a file over 100,000 characters is answered with `"truncated": true`, and a normal file's line has no `truncated` field (US2-4)
- [X] T023 [US2] Make the T022 tests pass: the pipeline carries the truncated flag from `file::read` into `Outcome::Answered` and `src/map.rs` writes it; fix whatever else the tests reveal in `src/pipeline.rs` or `src/map.rs`

**Checkpoint**: US2 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 5: User Story 3 - A leaner `filter` (Priority: P3)

**Goal**: failures on standard error by line number or input name only, truncation visible in the
summary; `--json`/`--all` were removed in Phase 2.

**Independent Test**: input with blank lines and a failing record: stderr names the right line and
never the record; `filter --json` is rejected.

- [X] T024 [P] [US3] Write `tests/pipeline/stderr.rs`, each for `filter` and `map`: a failing record after blank lines is reported with its line number counting the blanks, and across two files the numbering continues (US3-2, US3-3); a failing record of more than 100,000 characters never appears on stderr (SC-005); a named input file that does not exist and one that starts with a UTF-16 BOM are reported by file name, counted as failed, give no `map` output line, exit 2, and the next file's line numbers continue (edge cases); with `--read-files` and a truncated file the summary contains `1 truncated`, and without truncation the summary has no `truncated` (US3-4)
- [X] T025 [US3] Add the truncated count to `src/summary.rs` (counted from answered decisions with the truncated flag, rendered as `, N truncated` after `failed` only when N > 0, per contracts/cli.md) and fix whatever else T024 reveals

**Checkpoint**: US3 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T026 [P] Update `CLAUDE.md` (planned shape: subcommands `filter` and `map`, no `serve`) and `specs/manual/idea-draft.md` (no `serve`; the step loop runs one `map` per step; the "Later" list: caching, budgets, `-v`, sorting, other providers, per-record questions, structured state, the agent skill and examples)
- [X] T027 [P] Review `jevpipe map --help` against SC-008: an agent that knows `jq` can write a two-question triage pipeline from it alone; tighten help texts in `src/cli.rs`
- [X] T028 Run every command in quickstart.md against the real service with `OPENROUTER_API_KEY` and note surprises in `specs/002-map-command/quickstart.md`
- [ ] T029 Run `./check.ps1` (strict, `--locked`) and push the branch; CI passes on Linux, Windows and macOS (SC-007)

---

## Dependencies & Execution Order

- **Setup (T001–T002)** → **Foundational (T003–T014)** → **US1 (T015–T021)** → **US2 (T022–T023)** and
  **US3 (T024–T025)** → **Polish (T026–T029)**.
- Foundational: T003, T004, T005, T006 in parallel; T007 after T006; T008 after T003; T009 and T010 in
  parallel with the others; T011 after T004–T010; T012 and T013 after T011; T014 last, then the
  checkpoint.
- US1: T018 before T020; T019 before T021.
- US2 and US3 both need US1's `map`; they are independent of each other.

### Parallel Opportunities

- Phase 1: T002 alongside T001.
- Phase 2: T003, T004, T005, T006, T009, T010 (six files) in parallel.
- US1: T015, T016, T017 (three test files) in parallel; then T018 → T019 → T020 → T021.
- US2 tests (T022) and US3 tests (T024) can be written in parallel once US1 is done.
- Polish: T026 and T027 in parallel.

## Implementation Strategy

1. **Foundation first**: Phases 1–2 refactor the first milestone onto the shared engine; every `filter`
   test passing at the checkpoint shows the refactor kept its behaviour.
2. **MVP**: US1. Stop and verify: `printf 'a\nb\n' | jevpipe map q.json` works against the real
   service with three question types, all US1 tests pass.
3. **Increment**: US2 (`--read-files` for `map`), then US3 (standard error and truncation).
4. **Finish**: Polish, push, CI green on all three platforms.

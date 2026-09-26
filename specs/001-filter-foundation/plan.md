# Implementation Plan: Filter Foundation

**Branch**: `001-filter-foundation` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/001-filter-foundation/spec.md`

## Summary

`jevpipe filter <QUESTION> [FILE]...` judges each record (text line, JSON line or file) with one Jev
`noul` question via OpenRouter's System One endpoint and writes the kept records unchanged, in input
order, as a stream. Records flow through one async pipeline: read → validate → ask (up to 100 in
flight, retried on transient errors) → write, with a one-line summary and grep-like exit status. The
pipeline stages are separate modules so `map` and `serve` reuse reading, the service client and
output later.

## Technical Context

**Language/Version**: Rust 1.98 (edition 2024), pinned in `rust-toolchain.toml`
**Primary Dependencies**: `clap` 4.6 (derive), `tokio` 1.53, `reqwest` 0.13 (`json`), `futures` 0.3,
`backon` 1.6, `serde` 1.0 / `serde_json` 1.0, `thiserror` 2.0; dev: `assert_cmd`, `predicates`,
`wiremock` 0.6, `tempfile`
**Storage**: N/A (stateless)
**Testing**: `cargo test`; integration tests run the real binary against a `wiremock` stand-in of the
service; no network, no API key
**Target Platform**: Linux, Windows, macOS (CI matrix); single binary
**Project Type**: single CLI crate (library + thin binary)
**Performance Goals**: ~100 decisions/s at default concurrency; first kept record written within 2 s
of being read (SC-002, SC-003)
**Constraints**: bounded memory on unbounded input; at most 400 KB read per file; output
byte-identical to input; stdout written only through one writer (`print_stdout` lint)
**Scale/Scope**: one subcommand, three input/output modes, ~8 modules

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass: one subcommand; cache, budgets, other providers, `-v` and ranking deferred; no automatic rate adaptation or second truncation pass |
| II. Automated Verification | Pass: every user story has offline integration tests against the real binary; `check.ps1` gates format, clippy, tests and `cargo deny` |
| III. Reusable Components | Pass: input, service client, pipeline and output are separate modules with no `filter`-specific coupling in input or service |
| IV. Decide, Don't Act | Pass: jevpipe only asks and prints; no generation, no actions, no history |
| V. A Good Unix Citizen | Partial, justified: streaming line I/O (JSONL passes through unchanged), clean exit codes, one-line summary, broken-pipe handling are in; **caching and budgets are deferred** to the next milestone (see Complexity Tracking) |

Post-design re-check: unchanged; the design adds no project, no persistent state and no abstraction
beyond the modules listed below.

## Project Structure

### Documentation (this feature)

```text
specs/001-filter-foundation/
├── spec.md
├── api-spike.md         # live service measurements and verbatim responses
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md           # command, options, env, stdout/stderr formats, exit status
│   └── service.md       # System One request/response subset and error classes
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── main.rs        # parse CLI, start the tokio runtime, call run, map to ExitCode
├── lib.rs         # module declarations, `run`
├── cli.rs         # clap definitions: `filter` and its options, validation
├── record.rs      # Record, reading lines (raw bytes kept) from files and stdin
├── file.rs        # --read-files: skip rules, bounded read, truncation, file state
├── service.rs     # System One client: request/response types, error classes, retry and timeout
├── decision.rs    # Decision/Outcome, threshold
├── filter.rs      # the pipeline: records → buffered(concurrency) decisions → output + summary
├── output.rs      # stdout writer (raw / json / all), stderr failure lines, broken pipe
└── summary.rs     # counts, cost, duration, model, exit status

tests/
├── cli.rs            # --version, usage errors (no service needed)
└── filter/           # one test crate, so shared helpers never count as dead code
    ├── main.rs       # declares the modules below
    ├── stand_in.rs   # wiremock server + responder that answers from the record's content
    ├── lines.rs      # user story 1
    ├── streaming.rs  # streaming, broken pipe, throughput, timeouts
    ├── files.rs      # user story 2
    └── json.rs       # user story 3
```

**Structure Decision**: single crate; the library holds all logic (`CLAUDE.md` layout), `main.rs`
only wires. Integration tests live in one test crate, `tests/filter/`, which shares the stand-in.

### Other repository changes

- `Cargo.toml`: add the dependencies above.
- `deny.toml`: allow `ISC`, `BSD-3-Clause`, `MIT-0`, `CDLA-Permissive-2.0` (rustls stack; see
  research.md).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Constitution V: no cache and no budgets in this milestone | Principle I (lean MVP) and the agreed milestone cut; both are the next milestone and listed in `specs/manual/idea-draft.md` | Building them now doubles the scope of the first review and designs the cache key before the record model has been used |

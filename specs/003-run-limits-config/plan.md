# Implementation Plan: Run Limits, User Config and a Stored API Key

**Branch**: `003-run-limits-config` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/003-run-limits-config/spec.md`

## Summary

Three additions around the existing `filter`/`map` pipeline:

1. **Run limits.** `--max-cost` and `--max-time` bound one invocation. One shared `Limits` object
   holds the spend (in nano-dollars), the deadline and the stop reason; it is asked before each
   request. A stop ends the input, lets in-flight requests finish (spend) or abandons them (time),
   and the run prints a stop line with the resume line and exits 3. OpenRouter's 402 is split by `limit_source`: key limit and credits stop the run
   the same way, the in-flight budget is retried.
2. **A user config file.** The config-backed run options move into one `Settings` args struct; the
   TOML file (found via `JEVPIPE_CONFIG` or the user config directory) is loaded before parsing and
   its values become clap defaults, so flags win and `--help` shows effective values. `base-url`
   replaces `JEVPIPE_BASE_URL`. `config list|get|set|unset|path` read and edit the file with
   `toml_edit`.
3. **A stored API key.** `OPENROUTER_API_KEY` wins; otherwise the key comes from the OS keychain via
   `keyring-core` (Credential Manager, Secret Service, and on macOS our own store over
   `/usr/bin/security`). `auth set-key` reads one line from stdin with echo switched off on stdin
   itself and restores the terminal on Ctrl+C.

## Technical Context

**Language/Version**: Rust 1.98 (edition 2024), pinned in `rust-toolchain.toml`
**Primary Dependencies**: existing (`clap`, `tokio`, `reqwest`, `futures`, `backon`, `serde`,
`serde_json`, `thiserror`, `encoding_rs`); new: `humantime`, `toml_edit`, `etcetera`, `keyring-core`,
`windows-native-keyring-store` (Windows), `zbus-secret-service-keyring-store` (Linux),
`winapi-util` (Windows), `rustix` (Unix); clap feature `string`, tokio feature `signal`
**Storage**: the user config file (TOML) and one keychain entry; runs stay stateless
**Testing**: `cargo test`; binary tests against the `wiremock` stand-in with `JEVPIPE_CONFIG` pointing
at a temp file; keychain logic in-process against `keyring_core::mock::Store`; manual quickstart for
terminals and real keychains
**Target Platform**: Linux, Windows, macOS (CI matrix); single binary
**Project Type**: single CLI crate (library + thin binary)
**Performance Goals**: reading a stored key < 50 ms (measured 1–18 ms); with `--max-time T` the
process ends within T + 1 s
**Constraints**: the key never on stdout/stderr/files/argv; no `unsafe`; spend overshoot bounded by
the requests in flight; tests never touch the network, the real keychain or the user's config
**Scale/Scope**: two new subcommands (`config`, `auth`), two new run options, 10 new modules,
~1,000 new lines

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass: limits per invocation only (no accumulation, no token budget, no estimates); one user config file (no project config, no env var per setting); OpenRouter only; the key store is the OS keychain plus the env var, no OAuth, no key check |
| II. Automated Verification | Pass: every user story has offline tests (binary against the stand-in for limits and config; in-process mock store for the keychain, as the generalised testing rule in CLAUDE.md allows); `check.ps1` gates format, clippy, tests and `cargo deny`; terminals and real keychains are covered by the quickstart and were verified in the spike |
| III. Reusable Components | Pass: one `Settings` definition feeds flags, config validation, `config set` and `--help`; every keychain goes through `keyring_core::Entry`; one `Limits` object serves both commands through the shared pipeline |

Post-design re-check: unchanged. Linux gains 73 crates (zbus + RustCrypto) for the Secret Service;
accepted by the user as the cost of Linux keychain support, Linux builds only, licenses allowed.

## Project Structure

### Documentation (this feature)

```text
specs/003-run-limits-config/
├── spec.md
├── plan.md
├── research.md          # R1–R10: 402 cases, cost unit, stopping, durations, config, keychains, prompt, testing
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md           # run options, config file and commands, auth commands, stop lines, exit status
│   └── service.md       # base-url, cost, 402 classes
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── main.rs              # CHANGED: only calls `jevpipe::run()`; parsing moves into the library because it needs the config first
├── lib.rs               # CHANGED: modules; `run` loads the config, parses with its defaults, dispatches filter, map, config, auth
├── cli.rs               # CHANGED: `config` and `auth` subcommands; RunArgs = --read-files + flattened Settings; help texts
├── settings.rs          # NEW: Settings args (model, concurrency, request-timeout, max-cost, max-time) with their value parsers (duration via humantime, Limit<T> = none | value); keys; value check via the flag parser; config values as clap defaults
├── config/
│   ├── mod.rs           # NEW: the config file: path (JEVPIPE_CONFIG, etcetera), load + check (toml_edit), base-url, origins
│   └── command.rs       # NEW: config list | get | set | unset | path
├── auth/
│   ├── mod.rs           # NEW: API key lookup (env, then keychain); the key rules
│   ├── keychain.rs      # NEW: default store per platform; get / set / remove; errors
│   ├── security.rs      # NEW (macOS): keyring-core store over /usr/bin/security
│   ├── prompt.rs        # NEW: one line from stdin with echo off on stdin; Ctrl+C restores
│   └── command.rs       # NEW: auth set-key | remove-key
├── cost.rs              # NEW: Cost, an amount in nano-dollars: from the service's cost, from --max-cost, display
├── limits.rs            # NEW: Limits of one run: spend (atomic) against max-cost, the deadline, the stop reason (first wins), the stop line
├── pipeline.rs          # CHANGED: asks Limits before sending; Unprocessed; input ends at the stop; deadline arm; drain; resume line
├── service.rs           # CHANGED: ServiceConfig from base-url + key; 402 by limit_source; cost as Cost
├── summary.rs           # CHANGED: cost read from Limits; stop; Exit::Stopped (3), Exit::Interrupted (130); precedence
└── (answers, decision, file, filter, map, output, questions, reason, record, text: unchanged)

tests/
├── cli.rs               # CHANGED: help texts (limits, exit 3, config, auth); duration and cost usage errors; config file instead of JEVPIPE_BASE_URL
├── config.rs            # NEW: user story 2 through the binary (JEVPIPE_CONFIG in a temp dir)
├── auth.rs              # NEW: user story 3 paths that never reach a keychain (empty/invalid key, key as argument, env var wins)
└── pipeline/
    ├── stand_in.rs      # CHANGED: base-url via a temp config file; markers cost:<USD>, limit:key|credits, inflight:<times>
    ├── limits.rs        # NEW: user story 1 (spend, time, 402 cases, resume, exit precedence)
    └── …                # existing files: --request-timeout values get units
```

In-process tests: `#[cfg(test)]` modules in `src/auth/mod.rs` (lookup order, set/remove, errors) against
`keyring_core::mock::Store`, run as one sequential test because the default store is process-global.

**Structure Decision**: the crate grows from 16 to 26 module files, past the ~20 at which 002 said
subfolders pay off. The two self-contained areas become folders (`config/`, `auth/`); `settings`,
`cost` and `limits` stay flat next to the pipeline that uses them. Each new module is one entity of
the spec: flag values live with the flags in `settings`, and everything the pipeline needs to know
about limits is behind one `Limits` object.

### Other repository changes

- `Cargo.toml`: new dependencies above, target-specific where platform-only
  (`[target.'cfg(windows)'.dependencies]` etc.); `windows-native-keyring-store` and
  `zbus-secret-service-keyring-store` with `default-features = false`, the latter with
  `rt-async-io-crypto-rust`.
- `CLAUDE.md`: testing rule already generalised (done with the spec). At implementation: environment
  section (`JEVPIPE_BASE_URL` gone; `JEVPIPE_CONFIG`), shape (`config`, `auth`).
- `specs/manual/idea-draft.md`: budgets done (per invocation), caching dropped (identical requests are
  rare; provider-side input caching is not live for Jev), OpenRouter only.
- `.github/workflows/ci.yml`: unchanged (tests need no keychain or Secret Service).

## Code Structure

1. **One definition per run option.** `Settings` holds the five config-backed flags with their
   parsers and built-in defaults. `config` asks it for the keys, checks file values by parsing
   `--<key>=<value>` against it, and injects the file's values with `mut_arg(..).default_value(..)`.
   `base-url` is the only config key that is not a flag.
2. **Parse after loading.** `lib.rs`'s `run` loads the config (a `Result`), builds the clap command
   with the file's defaults when it loaded, parses and dispatches; `main.rs` stays wiring only. A config error is reported by the commands that need the
   file (`filter`, `map`, `config list`, `config get`); `config path|set|unset` and `--help` work
   without it.
3. **Limits are values, not special cases.** `Limit<T>` (`none` or a value) is what `--max-cost` and
   `--max-time` parse into, so "no limit" and "limit from the file, lifted by the flag" need no extra
   state.
4. **One `Limits` object per run.** The pipeline talks to limits through one shared object:
   `may_send()` right before `service.ask`, `add(cost)` right after, `stop(reason)` on a 402 limit,
   `stopped()` for the input's `take_while`, `deadline()` for the `select!` arm, and the stop line at
   the end. The summary reads the total spend from it, so cost stops flowing through `Outcome`.
5. **One stop path for three triggers.** Spend refusal and a 402 limit both return
   `Decided::Unprocessed` and set the stop reason; the input ends, the pipeline keeps draining
   in-flight work without printing after the first `Unprocessed`, then prints the stop line. The
   deadline breaks the same loop at once. For every trigger the resume line is the line after the
   last decided record (1 when none was): only blank lines can lie between it and the first
   unprocessed record, so rerunning from either gives the same result.
6. **Block only on plain threads.** Anything that blocks on stdin (the record reader, the key prompt)
   runs on a `std::thread`, so returning from `main` never waits for it (research R3, R9).
7. **Every keychain through `keyring-core`.** `keychain.rs` picks the store per platform
   (`#[cfg(...)]`), `security.rs` implements the store traits for macOS, and tests swap in the mock
   store; `keyring_core::Error` is matched with `matches!` because it is `#[non_exhaustive]`.
8. **The key is checked before the keychain is opened.** `auth set-key` reads and validates the line
   first; `run` resolves the key before reading input and opens the keychain only without
   `OPENROUTER_API_KEY`.
9. **Exit precedence in one place.** `Summary::exit` orders closed → stopped → failed → command rule.

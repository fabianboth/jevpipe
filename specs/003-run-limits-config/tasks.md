---
description: "Task list for run limits, user config and a stored API key"
---

# Tasks: Run Limits, User Config and a Stored API Key

**Input**: Design documents from `specs/003-run-limits-config/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/cli.md](contracts/cli.md),
[contracts/service.md](contracts/service.md), [quickstart.md](quickstart.md)

**Tests**: Required by FR-023. Integration tests run the real binary (`assert_cmd`) against the
`wiremock` stand-in, always with `JEVPIPE_CONFIG` pointing at a file in a temp dir (never the
developer's config); no network, no API key, no keychain. Keychain logic is tested in-process against
`keyring_core::mock::Store`, as CLAUDE.md's testing rule allows. Write each story's tests first and see
them fail.

**Rules for every task**: follow `CLAUDE.md` (no comments, max 3 parameters besides `self`,
exhaustive `match` without `_` on enums, `matches!` for the `#[non_exhaustive]` `keyring_core::Error`,
no `unwrap`/`expect`/`panic!` in `src/`, no `unsafe`, private by default, one module per concept).
clap help text goes in doc comments on the clap types. Anything that blocks on stdin runs on a
`std::thread`, never `spawn_blocking` (plan.md point 6). `./check.ps1 -Fix` passes at the end of each
phase. "Point N" refers to plan.md, "Code Structure"; "R N" to research.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1 limits, US2 config, US3 stored key)

---

## Phase 1: Setup

**Purpose**: dependencies.

- [X] T001 Add the dependencies with `cargo add` in `Cargo.toml` (plan.md Technical Context, R4–R9): `humantime@2.4`, `toml_edit@0.25`, `etcetera@0.11`, `keyring-core@1`; `--target 'cfg(windows)'`: `windows-native-keyring-store@1.1 --no-default-features`, `winapi-util@0.1`; `--target 'cfg(target_os = "linux")'`: `zbus-secret-service-keyring-store@1 --no-default-features --features rt-async-io-crypto-rust`; `--target 'cfg(unix)'`: `rustix@1 --features termios,stdio`; clap feature `string`; tokio feature `signal`. Run `cargo deny check` and confirm it passes the existing `deny.toml` (only `multiple-versions` warnings, e.g. `syn`)

**Checkpoint**: `./check.ps1 -Fix` passes with the unchanged product.

---

## Phase 2: Foundational

**Purpose**: what every story and every test needs: the `Settings` flags with their new value types,
the `Cost` amount, and the config file as the only way to set the service address (FR-009), so the
test stand-in is reached through `base-url`. **No story work before this phase is done.**

- [X] T002 [P] Create `src/cost.rs` (data-model.md Cost, R2): `Cost` as whole nano-dollars (`u64`); from a service cost (`f64` dollars, rounded to the nearest nano-dollar, negative or non-finite rejected); a `FromStr` for `--max-cost` values (a positive decimal number of dollars, at most 9 decimals); `Display` as dollars with up to 6 decimals and trailing zeros trimmed, matching today's `summary.rs` `dollars()` (`0.001`, `0.5012`); addition. Move the summary's dollar formatting here
- [X] T003 [P] Create `src/settings.rs` (data-model.md Settings, Limit, Duration; point 3): a `#[derive(Args)]` `Settings` with `model` (default `~typesafe/jev-latest`), `concurrency` (`NonZeroUsize`, default `100`), `request_timeout` (`Duration`, default `10s`), `max_cost` (`Limit<Cost>`, default `none`), `max_time` (`Limit<Duration>`, default `none`), doc comments per contracts/cli.md; `Limit<T>` = `none` or a value, with its value parser; the duration parser via `humantime::parse_duration`, rejecting zero and rewording errors (`needs a unit, for example 10s or 5m` for a bare number), displayed with `humantime::format_duration`
- [X] T004 Update `src/cli.rs`: `RunArgs` keeps `--read-files` and flattens `Settings`; remove the old `concurrency`, `model` and `request_timeout` fields; update their uses in `src/pipeline.rs` (`request_timeout` is now a `Duration`)
- [X] T005 Create `src/config/mod.rs` (R6, data-model.md Config file; the loading half, origins and editing come in US2): the file path (`JEVPIPE_CONFIG` when set, else `etcetera::choose_base_strategy()` config dir + `jevpipe/config.toml`); load with `toml_edit::DocumentMut` (a missing file is empty); check every key (the `Settings` long names plus `base-url`) and value: settings values by parsing `--<key>=<value>` with a command built from `Settings::augment_args` (R5), any scalar accepted and turned into text; `base-url` parsed as an absolute `http`/`https` URL (default `https://openrouter.ai/api`); errors name the file, the key and, for unknown keys, the valid keys (contracts/cli.md "Config errors")
- [X] T006 Wire the config into the run (point 2): `src/main.rs` only calls `jevpipe::run()`; `src/lib.rs`'s `run` loads the config (a `Result`), parses the command line, and for `filter`/`map` reports a config error as `jevpipe: error: …` with exit 2 before reading input; `src/service.rs`'s `ServiceConfig` takes the base URL from the config instead of `JEVPIPE_BASE_URL` (removed) and the key from `OPENROUTER_API_KEY` as today; `Reply.cost` becomes `Option<Cost>` and `src/summary.rs` uses `Cost`
- [X] T007 Update the tests to the changed setup: in `tests/pipeline/stand_in.rs` write a config file with `base-url = "<stand-in uri>"` into a temp dir and set `JEVPIPE_CONFIG` to it instead of `JEVPIPE_BASE_URL`; give every other binary test (`tests/cli.rs`) a `JEVPIPE_CONFIG` pointing at a missing file in a temp dir (a shared helper); change every `--request-timeout <N>` in `tests/` to `<N>s`; add to `tests/cli.rs`: `--request-timeout 10` and `--request-timeout 0s` are usage errors naming the unit rule (FR-008); a config file with an unknown key or `concurrency = 0` makes `filter` fail with exit 2 before any request, naming the file and key (FR-012)

**Checkpoint**: every existing test passes through the config file; `./check.ps1 -Fix` passes;
`JEVPIPE_BASE_URL` no longer appears in `src/` or `tests/`.

---

## Phase 3: User Story 1 - Guard one run against runaway spend and time (Priority: P1) 🎯 MVP

**Goal**: `--max-cost` and `--max-time` stop a run with a valid prefix, a stop line naming the limit
and the resume line, and exit 3; OpenRouter's key-limit and credits 402 stop the same way; the
in-flight 402 is retried.

**Independent Test**: 100 lines through `map` with `--max-cost` at 10 requests' cost and
`--concurrency 1`: the first answers, the stop line with the resume line, exit 3; rerunning from that
line completes the output.

### Tests for User Story 1

- [X] T008 [US1] Extend `tests/pipeline/stand_in.rs` with markers (R10), in the existing `name=value` style: `cost=<usd>` (the answer's `usage.cost`; default stays `0.00001`), `nocost` (no `usage.cost`), `limit=key_limit` and `limit=credits` (402 with `error.metadata.limit_source` `openrouter_key_limit` / `openrouter_credits`, body per contracts/service.md), `inflight=<times>` (402 with `limit_source` `openrouter_in_flight_budget` and `Retry-After: 0` for the first `<times>` attempts), the existing `status=402` (402 without `metadata`); keep `slow=<ms>` for delays
- [X] T009 [US1] Write `tests/pipeline/limits.rs`, one test per behaviour: US1-1 (spend limit, `--concurrency 1`: exact output prefix, the stop line text, exit 3); US1-2 (several in flight when the limit is reached: their records are printed when every earlier one was, the summary's cost includes them, no request after the stop); US1-3 (`--max-time 2s`, `slow:1000`, `--concurrency 1`: exits within 3 s, stop line with the time limit, exit 3; SC-002); US1-4 (stdin held open without input, `--max-time 1s`: exits 3 within 2 s); US1-5 and SC-003 (joining the stopped output and a rerun from the resume line equals an uninterrupted run, for `filter` and `map`, including blank lines and two input files); US1-6 (`limit=key_limit` and `limit=credits`: stop lines naming each, exit 3); US1-7 and SC-004 (`inflight:2` is retried and answered, run not stopped); US1-8 (`--max-cost` with `nocost`: `jevpipe: error: the service reported no cost, so --max-cost cannot be enforced`, exit 2); US1-9 (a limit not reached: no stop line, exit as without limits); `402` without metadata: run-level error, exit 2; skipped records before the stop are printed and the resume line comes after them; a failed record and a stop in one run exit 3 (FR-007); `--max-cost none` behaves as no limit
- [X] T010 [P] [US1] Add to `tests/cli.rs`: `--max-cost 0`, `-1`, `abc` and `--max-time 10`, `0s` are usage errors with exit 2 before any input is read; `filter --help` and `map --help` list `--max-cost <DOLLARS|none>` and `--max-time <DURATION|none>` and describe exit status 3

### Implementation for User Story 1

- [X] T011 [US1] Classify 402s in `src/service.rs` (R1, contracts/service.md): read `error.metadata.limit_source` (optional) from the error body; `openrouter_in_flight_budget` → `Transient { retry_after }` from `Retry-After`; `openrouter_key_limit` / `openrouter_credits` → a new `ServiceError::Limit(Limit::KeyLimit | Limit::Credits)`; anything else stays `Rejected`
- [X] T012 [US1] Create `src/limits.rs` (data-model.md Limits and Stop; point 4): `Limits` with `spent` (`AtomicU64` nano-dollars, plus whether any cost was reported), `max_cost`, `deadline` (start + `max_time`), `stop` (`OnceLock<Stop>`, first wins); `may_send()` (false once stopped or `spent ≥ max_cost`, setting `Stop::SpendLimit` in the latter case); `add(Option<Cost>)` (error when `max_cost` is set and the cost is missing, FR-006); `stop(Stop)`; `stopped()`; `deadline()`; `spent()`; the stop line `stopped: <reason>; input from line L on was not processed` with the reason texts from contracts/cli.md
- [X] T013 [US1] Stop runs in `src/pipeline.rs` (point 5, R3): build one shared `Limits` per run; in `Judge::ask` call `may_send()` right before `service.ask` (false → `Decided::Unprocessed`), `add(reply.cost)` right after (its error is a `RunError`), `ServiceError::Limit` → `limits.stop(..)` and `Decided::Unprocessed`; end the input stream with `take_while(!limits.stopped())`; in the decision loop `select!` on the next decision and `sleep_until(deadline)` when a deadline is set (deadline → `Stop::TimeLimit`, break, dropping in-flight work); track the last decided record's line; after the first `Unprocessed` print nothing more but keep draining; at the end print the stop line with resume line = last decided line + 1 (1 when none) before the summary, only when a record was left unprocessed or the deadline fired
- [X] T014 [US1] Update `src/summary.rs` (point 9, data-model.md Exit): the cost comes from `Limits` (shown when any cost was reported), not from outcomes; `Exit::Stopped` (status 3); `Summary::exit` precedence: output closed → 0; stopped → 3; failed or run-level error → 2; then the command's rule
- [X] T015 [US1] Help texts in `src/cli.rs`: the exit status paragraphs of `filter` and `map` include 3 (a limit stopped the run; the output is a prefix; stderr names the line to resume from), per contracts/cli.md

**Checkpoint**: US1 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 4: User Story 2 - Set defaults once in a user config file (Priority: P2)

**Goal**: config file values become the run defaults (flags win, `--help` shows them), and
`config list|get|set|unset|path` read and edit the file.

**Independent Test**: with `JEVPIPE_CONFIG` in a temp dir, `config set max-cost 0.5` and
`config set concurrency 4`, then a `map` run limited accordingly; `config list` shows both from the
config file; `--concurrency 8` overrides for one run.

### Tests for User Story 2

- [X] T016 [US2] Write `tests/config.rs` (binary, `JEVPIPE_CONFIG` in a temp dir, `OPENROUTER_API_KEY` set so no keychain is opened), one test per behaviour: US2-1 (`config set model …` creates the file and its directory; a run against the stand-in sends that model); US2-2 (set and unset keep other keys and comments, byte-compared); US2-3 (a flag beats the file for one run); US2-4 (`config set concurrency 0` / `max-time 10` rejected with the flag's message, file unchanged); US2-5 (unknown key in the file: `config list` and a run fail with file, key and valid keys, exit 2); US2-6 (with a broken file `config path`, `set` and `unset` work; `unset` removes an unknown key); US2-7 (`config list` prints valid TOML with every key, its effective value and `# default` / `# config file`, and `# API key: from OPENROUTER_API_KEY`); US2-8 (`config get` prints only the value: `100`, `none`, `20s`); US2-9 (`config path` prints the `JEVPIPE_CONFIG` path whether or not it exists); US2-10 (`max-cost` in the file, `--max-cost none` on the run: no stop); US2-11 (`map --help` shows `[default: 4]` for a configured concurrency, and the built-in defaults with a broken file); SC-005 (after `config set` of model, concurrency, request-timeout, max-cost, max-time, a run needs no flags); `base-url` from the file is used and a non-http URL is rejected

### Implementation for User Story 2

- [X] T017 [US2] Config values as clap defaults (point 1, R5): in `src/settings.rs`, a function that takes the clap `Command` and the loaded config and sets each configured key's value as the default of the matching argument on the `filter` and `map` subcommands (`mut_subcommand` + `mut_arg(id).default_value(..)`, looking the id up by long name; ids never come from the file unchecked); `src/lib.rs` parses with it (`FromArgMatches`) when the config loaded, with the plain command otherwise (so `--help` works with a broken file)
- [X] T018 [US2] Origins and editing in `src/config/mod.rs` (R6): each key's effective value and origin (`config file` or `default` from `Arg::get_default_values` on the unmodified command); `set` (validated like the flag, creates the directory and file, keeps the old value's decor, writes an integer, float or string, whichever parses first); `unset` (removes any key present, even an unknown one); both work on the raw document of a broken file
- [X] T019 [US2] Create `src/config/command.rs` with `list`, `get`, `set`, `unset`, `path` per contracts/cli.md: `list` prints aligned TOML lines with `# default` / `# config file` and a last line `# API key: from OPENROUTER_API_KEY` or `# API key: not set` (the keychain source is added in US3); `get` prints the bare value; errors exit 2
- [X] T020 [US2] Add the `config` subcommand with `list`, `get <KEY>`, `set <KEY> <VALUE>` (value with `allow_hyphen_values`), `unset <KEY>`, `path` to `src/cli.rs` (help: the keys, the file location, `JEVPIPE_CONFIG`, precedence) and dispatch it in `src/lib.rs`; `list`/`get` report a config load error, `path`/`set`/`unset` do not need a loadable file

**Checkpoint**: US2 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 5: User Story 3 - Store the API key once (Priority: P3)

**Goal**: `auth set-key` / `auth remove-key` keep the key in the OS keychain; runs use
`OPENROUTER_API_KEY` first, then the keychain.

**Independent Test**: in-process with the mock store: store a key, resolve it without the variable,
the variable wins when set, remove it and get the "no API key" error; through the binary: the paths
that never reach a keychain.

### Tests for User Story 3

- [X] T021 [P] [US3] Write the in-process tests as one sequential `#[test]` in a `#[cfg(test)]` module of `src/auth/mod.rs`, with `keyring_core::set_default_store(keyring_core::mock::Store::new()?)` (R10): no key anywhere → the "no API key" error naming both ways; a stored key is resolved when the variable is unset or empty (US3-5, edge case); a non-empty variable wins and the store is not accessed (US3-6; check by resolving it without any default store set, before the mock is installed); set replaces a stored key; remove, and remove with nothing stored succeeds (US3-9); a mocked store error maps to "keychain unavailable" with the store's message (US3-8)
- [X] T022 [P] [US3] Write `tests/auth.rs` (binary; paths that never open a keychain): `echo "" | jevpipe auth set-key` → `no API key given`, exit 2 (US3-4); keys with a space, a `"`, a `\`, a non-ASCII character or a zero-width space (U+200B) → the invalid-key message, exit 2; `auth set-key sk-or-v1-abc` → usage error (FR-016); piped input shows no prompt on stderr; `config list` with `OPENROUTER_API_KEY` set says `from OPENROUTER_API_KEY` and never contains the key (FR-021, SC-007); `auth --help` describes both commands and the piped form

### Implementation for User Story 3

- [X] T023 [P] [US3] Create `src/auth/prompt.rs` (R9, point 6): an echo guard on stdin itself (Windows: `winapi_util::console::mode`/`set_mode` on `HandleRef::stdin()` clearing `ENABLE_ECHO_INPUT` 0x0004; Unix: `rustix::termios` on `rustix::stdio::stdin()` removing `LocalModes::ECHO`), restoring the exact previous state on `Drop`; `read_key()`: when `std::io::IsTerminal` says stdin is a terminal, print `OpenRouter API key: ` to stderr (`OpenRouter API key (input will be visible): ` when echo could not be switched off) and a newline after the input; read one line on a `std::thread` sending it through a `tokio::sync::oneshot`; `select!` with `tokio::signal::ctrl_c()`: on Ctrl+C drop the guard, print `interrupted`, and return an interrupted result without waiting for the read (the prototype in `scratch/echoproto` is the reference)
- [X] T024 [P] [US3] Create `src/auth/keychain.rs` (R7, point 7): install the platform's default store once (`#[cfg(windows)]` `windows_native_keyring_store::Store::new()`, `#[cfg(target_os = "linux")]` `zbus_secret_service_keyring_store::Store::new()`, `#[cfg(target_os = "macos")]` the store from T025); `Entry::new("jevpipe", "openrouter-api-key")`; `get` → key, none or unavailable; `set`; `remove` → removed or none; `keyring_core::Error::NoEntry` via `matches!`, every other error as unavailable with its message; a store that fails to open is unavailable
- [X] T025 [P] [US3] Create `src/auth/security.rs` (`#[cfg(target_os = "macos")]`, R8): a `keyring_core` credential store (`CredentialStoreApi` + `CredentialApi`) over `/usr/bin/security` via `std::process::Command`: set runs `security -i` and writes `add-generic-password -U -s <service> -a <user> -w "<key>"` plus a newline to its stdin (never in argv); get runs `find-generic-password -s <service> -a <user> -w` and trims stdout; delete runs `delete-generic-password -s <service> -a <user>`; exit status 44 → `NoEntry`, other failures → `NoStorageAccess`/`PlatformFailure` with stderr's message. Verify it compiles with `cargo check --target aarch64-apple-darwin` (after `rustup target add aarch64-apple-darwin`); the macOS CI job compiles and runs the suite
- [X] T026 [US3] Create `src/auth/mod.rs` (data-model.md API key): the key rules (trimmed, non-empty, printable ASCII without spaces, `"`, `'`, `\`) with the messages from contracts/cli.md; key resolution: `OPENROUTER_API_KEY` when non-empty (keychain not opened), else the keychain, else the "no API key: set OPENROUTER_API_KEY or run jevpipe auth set-key" error; the key's source for `config list` (environment, keychain, not set, keychain unavailable with reason); the key value is never formatted into any message
- [X] T027 [US3] Create `src/auth/command.rs`: `set-key` reads with T023, checks the rules before opening the keychain (point 8), stores it and prints `jevpipe: API key stored in the keychain` to stderr; `remove-key` prints `jevpipe: API key removed` or `jevpipe: no API key stored`; no keychain → `jevpipe: error: no keychain available here (<reason>); set OPENROUTER_API_KEY instead`, exit 2; interrupted → exit 130
- [X] T028 [US3] Wire it: `src/cli.rs` `auth` subcommand with `set-key` and `remove-key` (no key argument; help with the piped form and the key order); dispatch in `src/lib.rs`; `src/summary.rs` `Exit::Interrupted` (130); `src/service.rs`/`src/pipeline.rs` take the resolved key from `auth` (resolved before any input is read); `src/config/command.rs` shows the keychain source in `config list`'s API key line

**Checkpoint**: US3 tests pass; `./check.ps1 -Fix` passes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T029 [P] Update `CLAUDE.md` (shape: `filter`, `map`, `config`, `auth`; the environment: `OPENROUTER_API_KEY`, `JEVPIPE_CONFIG`, no `JEVPIPE_BASE_URL`) and `specs/manual/idea-draft.md` (budgets per invocation done; caching dropped, identical requests are rare and provider-side input caching is not live for Jev; OpenRouter only; the config file and stored key)
- [X] T030 [P] Review every `--help` against FR-022 and contracts/cli.md (new options with `none`, exit status 3, config commands, keys and file location, the ways to provide the key); tighten texts in `src/cli.rs`
- [X] T031 Run the quickstart's manual checks on this Windows machine (hidden prompt in Windows Terminal/VS Code, PowerShell, cmd and the Git Bash window, Ctrl+C; the key in Credential Manager; a guarded run with `--max-cost` against the real service) and note results in `specs/003-run-limits-config/quickstart.md`; list the macOS and Linux desktop checks as still to be done by hand
- [ ] T032 Run `./check.ps1` (strict, `--locked`) and push the branch; CI passes on Linux, Windows and macOS (SC-009)

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T007)** → **US1 (T008–T015)**, **US2 (T016–T020)** and
  **US3 (T021–T028)** → **Polish (T029–T032)**.
- Foundational: T002 and T003 in parallel; T004 after T003; T005 after T003; T006 after T002, T004,
  T005; T007 last, then the checkpoint.
- US1: T008 before T009; T011 and T012 before T013; T014 after T012.
- US2: T017 and T018 before T019; T020 last.
- US3: T023, T024, T025 in parallel; T026 after T024; T027 after T023 and T026; T028 last. T028's
  `config list` part needs US2's `src/config/command.rs` (T019); if US3 is done before US2, do that
  part when T019 lands.
- The three stories are otherwise independent of each other after Phase 2.

### Parallel Opportunities

- Phase 2: T002 and T003.
- US1: T010 alongside T008/T009; T011 alongside T012.
- US2: T017 alongside T018.
- US3: T021 and T022 (tests), then T023, T024 and T025 (three platform-facing modules).
- Different stories can proceed in parallel after Phase 2 (they touch `src/cli.rs` and `src/lib.rs`
  in small, separate places).
- Polish: T029 and T030.

## Implementation Strategy

1. **Foundation first**: Phases 1–2 move the run options into `Settings`, add `Cost`, and route the
   service address through the config file; every existing test passing through `JEVPIPE_CONFIG`
   shows nothing else changed.
2. **MVP**: US1. Stop and verify: a `--max-cost` run against the real service stops with exit 3 and a
   resume line that continues the run.
3. **Increment**: US2 (config defaults and commands), then US3 (the stored key).
4. **Finish**: Polish, manual checks, push, CI green on all three platforms.

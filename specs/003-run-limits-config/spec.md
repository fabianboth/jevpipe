# Feature Specification: Run Limits, User Config and a Stored API Key

**Feature Branch**: `003-run-limits-config`
**Created**: 2026-09-27
**Status**: Draft
**Input**: User description: "jevpipe 003: run limits, user config and a stored API key. See conversation for all decisions: per-invocation --max-cost (soft stop) and --max-time (hard stop), exit status 3 when a limit stops the run (including OpenRouter key limit / credits 402), retry the transient in-flight-budget 402, OpenRouter only, base_url as config key, user-level config file with config list/get/set/unset/path, humantime durations, auth set-key/remove-key backed by the OS keychain with OPENROUTER_API_KEY taking precedence, keychain mocked in-process via keyring-core's mock store (CLAUDE.md rule adapted narrowly)."

## Context

jevpipe runs unattended: a coding agent writes a pipeline, and one command may send thousands of
requests. The spend limit on an OpenRouter API key protects the account over a month; nothing protects
against a single invocation that turns out far larger or slower than intended, for example an agent
pointing `--read-files` at a whole monorepo. This milestone adds two limits per invocation, spend and
wall-clock time, and makes every early stop legible to the agent: a distinct exit status, one message
naming the limit, and the input line where a rerun can resume without paying twice.

It also removes the need to repeat settings on every command. A user-level config file holds the
defaults for the run options (model, concurrency, timeouts, limits) and the service address, and the
API key can be stored once in the operating system's keychain instead of being exported in every shell.

jevpipe speaks OpenRouter's System One API only. The service address stays configurable for gateways
that speak the same API (and for the test stand-in); TypeSafe's own endpoint, which uses other model
names and reports no cost, is not supported.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Guard one run against runaway spend and time (Priority: P1)

A developer or agent adds `--max-cost 0.50` or `--max-time 10m` to a `filter` or `map` run. If the run
reaches the limit, jevpipe stops, keeps the output produced so far (a valid prefix in input order), and
tells the caller which limit stopped it and from which input line a rerun continues. The same happens
when OpenRouter itself refuses further requests because the key's spend limit or the account's credits
are used up.

**Why this priority**: It is the safety net for unattended use and the main reason for this milestone:
one invocation can never silently cost more or take longer than the caller allowed, and the caller can
tell an incomplete run from a complete or failed one.

**Independent Test**: Against a stand-in service that reports a fixed cost per request, run `map` over
100 lines with `--max-cost` set to the cost of 10 requests and `--concurrency 1`; the output holds the
first answered lines, standard error names the spend limit and the first unprocessed line, and the exit
status is 3. Rerunning over the input from that line produces the remaining lines, and both outputs
together equal an uninterrupted run.

**Acceptance Scenarios**:

1. **Given** `--max-cost 0.001` and a stand-in reporting $0.0001 per request with `--concurrency 1`, **When** `map` runs over 100 lines, **Then** no request is sent once the reported total reaches $0.001, the output holds exactly the lines decided before that point in input order, standard error holds `jevpipe: stopped: spend limit $0.001 reached ($0.001 spent); input from line 11 on was not processed`, and the exit status is 3.
2. **Given** `--max-cost` and several requests in flight when the limit is reached, **When** they complete, **Then** their records are still output if every earlier record was, the reported spend includes them, and nothing further is sent.
3. **Given** `--max-time 2s` and a stand-in that answers each request after 1 second with `--concurrency 1`, **When** `filter` runs over 10 lines, **Then** the process ends within 3 seconds, requests still in flight are abandoned, the output holds the kept lines decided before the stop, standard error names the time limit and the resume line, and the exit status is 3.
4. **Given** `--max-time` and a driver that writes no further input, **When** the time limit passes while jevpipe waits for input, **Then** it stops the same way.
5. **Given** a stopped run and its resume line L, **When** the caller reruns the same command over the input from line L on, **Then** the combined output equals that of one uninterrupted run.
6. **Given** OpenRouter answers with "payment required" because the API key's own spend limit or the account's credits are used up, **When** the run meets it, **Then** it stops as for `--max-cost` (prefix output, resume line, exit status 3) and the message says which of the two is used up.
7. **Given** OpenRouter answers with "payment required" because too many requests are in flight at once (a temporary budget, with a wait time), **When** the run meets it, **Then** the request is retried after the given wait like any other temporary failure, and the run is not stopped.
8. **Given** `--max-cost` and a service that reports no cost for an answer, **When** that answer arrives, **Then** the run stops with a run-level error saying the spend limit cannot be enforced, exit status 2.
9. **Given** a run that finishes all its input before a limit is reached, **Then** nothing about limits is printed and the exit status is as without limits.

---

### User Story 2 - Set defaults once in a user config file (Priority: P2)

A developer sets their usual model, a spend limit and a longer request timeout once with
`jevpipe config set`, and every later `filter` and `map` run uses them unless a flag says otherwise.
An agent that wants to know what applies runs `jevpipe config list`.

**Why this priority**: It makes the P1 guard practical (a spend limit configured once protects every
agent-written command) and removes repetition, but runs work without it.

**Independent Test**: With `JEVPIPE_CONFIG` pointing at a temporary file, run `config set max-cost 0.5`
and `config set concurrency 4`, then a `map` run against the stand-in; the run is limited to $0.50 and
never has more than 4 requests in flight; `config list` shows both values as coming from the config
file and every other key as a default; a `--concurrency 8` flag overrides the file for one run.

**Acceptance Scenarios**:

1. **Given** no config file, **When** the user runs `jevpipe config set model typesafe/jev-1.13`, **Then** the file and its directory are created with that value, and later runs use that model.
2. **Given** a config file with comments and other keys, **When** the user runs `config set` or `config unset`, **Then** only that key changes and the other keys and comments are preserved.
3. **Given** a value in the config file and the same option as a flag, **When** a run starts, **Then** the flag wins for that run.
4. **Given** `config set concurrency 0` or `config set max-time 10`, **When** it runs, **Then** it is rejected with the same message the flag would give, and the file is unchanged.
5. **Given** a config file with an unknown key (for example `concurency`), **When** a run or `config list` starts, **Then** it fails with a message naming the file and the key and listing the valid keys, exit status 2, before any input is read.
6. **Given** a broken config file, **When** the user runs `config path`, `config set` or `config unset`, **Then** these still work, so the file can be repaired; `config unset` accepts any key present in the file, including an unknown one.
7. **Given** any config, **When** the user runs `config list`, **Then** each key is printed with its effective value and whether it comes from the config file or the built-in default, plus where the API key comes from (environment, keychain, or not set) without ever showing the key.
8. **Given** a key, **When** the user runs `config get <key>`, **Then** only its effective value is printed, for use in scripts.
9. **Given** `JEVPIPE_CONFIG` set to a path, **When** any command runs, **Then** that file is used instead of the default location; `config path` prints the file in use whether or not it exists.
10. **Given** a configured `max-cost`, **When** one run needs no spend limit, **Then** `--max-cost none` lifts it for that run (likewise `--max-time none`).
11. **Given** a configured value, **When** the user runs `jevpipe map --help`, **Then** the default shown for that option is the configured value.

---

### User Story 3 - Store the API key once (Priority: P3)

A developer runs `jevpipe auth set-key`, pastes the OpenRouter key at a hidden prompt, and from then on
runs work in any new shell without exporting `OPENROUTER_API_KEY`. The environment variable still wins
where it is set, so CI, containers and cloud sessions with an injecting proxy keep working unchanged.

**Why this priority**: A convenience on the developer's own machine; the environment variable already
works everywhere.

**Independent Test**: With a stand-in keychain, store a key with `auth set-key` from standard input,
check a run resolves that key when `OPENROUTER_API_KEY` is unset and the environment's key when it is
set, then `auth remove-key` and check a run fails with the "no API key" message naming both options.

**Acceptance Scenarios**:

1. **Given** a terminal, **When** the user runs `jevpipe auth set-key`, **Then** a prompt is shown on standard error, the key typed or pasted on standard input is not shown, it is stored in the operating system's keychain, and a confirmation without the key is printed.
2. **Given** piped input such as `echo "$KEY" | jevpipe auth set-key`, **When** it runs, **Then** no prompt is shown, one line is read, surrounding whitespace and the line ending are removed, and the key is stored.
3. **Given** the prompt, **When** the user presses Ctrl+C, **Then** nothing is stored, the terminal shows typed characters again as before, and the exit status is 130.
4. **Given** empty input, or a key with a space, a quote, a backslash or a non-ASCII or invisible character (for example a zero-width space from a paste), **When** `auth set-key` runs, **Then** it fails with a message, stores nothing, and exits with status 2.
5. **Given** a stored key and no `OPENROUTER_API_KEY`, **When** a run starts, **Then** the stored key is used.
6. **Given** a stored key and a non-empty `OPENROUTER_API_KEY`, **When** a run starts, **Then** the environment's key is used and the keychain is not accessed.
7. **Given** neither, **When** a run starts, **Then** it fails before reading input with a message naming `OPENROUTER_API_KEY` and `jevpipe auth set-key`, exit status 2.
8. **Given** a machine without a usable keychain (for example a headless Linux server), **When** `auth set-key` runs, **Then** it fails with a message saying no keychain is available and to set `OPENROUTER_API_KEY` instead; runs with the environment variable are unaffected.
9. **Given** a stored key, **When** the user runs `jevpipe auth remove-key`, **Then** it is removed; with no stored key the command says so and exits 0.
10. **Given** a key stored by one jevpipe build, **When** jevpipe is updated or rebuilt, **Then** the new build reads, replaces and removes the stored key without any confirmation dialog, on every platform.

---

### Edge Cases

- **A limit below the cost of one request**: the first requests (up to `--concurrency`) are sent before any cost is known; the run then stops. The overshoot of `--max-cost` is bounded by the requests in flight when it is reached.
- **Records answered after the stop point**: when several requests are in flight, a later record may be answered while an earlier one is not; output stops at the first record without an outcome, so later answered records are not printed (their cost is counted and reported).
- **Skipped or failed records before the stop**: printed and counted as usual; they are decided, so the resume line comes after them and a rerun does not repeat them.
- **Failures and a limit in the same run**: the exit status is 3 (the output is incomplete); per-record failures are still reported on standard error by line number.
- **The output consumer goes away before a limit is reached**: as today, jevpipe stops without an error and exits 0.
- **A limit reached after the last record was sent**: no record is left unprocessed, so the run ends normally.
- **Several input files**: the resume line counts over all inputs joined in order, as line numbers already do for failures.
- **Invalid limit values**: `--max-cost` must be a positive number of US dollars or `none`; `--max-time` and `--request-timeout` must be a positive duration with a unit (`90s`, `10m`, `1h30m`) or, for the limits, `none`; anything else is a usage error before any input is read.
- **"Payment required" without an indication of which limit**: a run-level error with the service's message, exit status 2, as today.
- **Config file missing**: treated as empty; built-in defaults apply.
- **Config file not valid TOML, or a value of the wrong type**: error naming the file (and the key where known), exit status 2, for runs, `config list` and `config get`.
- **An empty `OPENROUTER_API_KEY`**: treated as unset, so the stored key is used.
- **A keychain that is present but locked or denies access**: the run fails with the keychain's reason and a hint to set `OPENROUTER_API_KEY`, exit status 2.
- **A terminal that cannot hide typing** (for example Git Bash's own window without pseudo console support): the prompt says the input will be visible, and the key is read the same way.

## Requirements *(mandatory)*

### Functional Requirements

**Run limits (both commands)**

- **FR-001**: `filter` and `map` MUST accept `--max-cost <DOLLARS|none>`: once the total cost reported by the service for this invocation reaches the limit, no further request is sent; requests already in flight complete. There is no default limit.
- **FR-002**: `filter` and `map` MUST accept `--max-time <DURATION|none>`: once this much wall-clock time has passed since the invocation started, the run stops at once, abandoning requests in flight and no longer reading input. There is no default limit.
- **FR-003**: Limits MUST apply to one invocation only; nothing is accumulated across invocations.
- **FR-004**: When a limit stops a run while input remains unprocessed, the output MUST be the in-order prefix of decided records up to the first record without an outcome, standard error MUST hold one line `jevpipe: stopped: <which limit and its value>[, amount spent]; input from line <L> on was not processed` before the summary line, and the exit status MUST be 3. `L` is the line after the last decided record (1 when none was), counted as for failures, so rerunning over the input from line `L` continues the run.
- **FR-005**: A "payment required" response that OpenRouter marks as the API key's own spend limit or the account's credits being used up MUST stop the run as in FR-004, with a message naming which one; one marked as the temporary in-flight budget MUST be retried like other temporary failures, honouring the wait the service asks for; any other "payment required" response is a run-level error as today.
- **FR-006**: With `--max-cost` set, an answer without a reported cost MUST stop the run with a run-level error saying the limit cannot be enforced.
- **FR-007**: Exit status precedence MUST be: the output consumer went away → 0; stopped by a limit → 3; a record failed or a run-level error → 2; otherwise as in the first two milestones.
- **FR-008**: `--request-timeout` MUST take a duration with a unit (default `10s`), the same format as `--max-time`; a bare number is rejected.

**Service**

- **FR-009**: jevpipe MUST speak OpenRouter's System One API only; the service address MUST default to OpenRouter and be changeable only through the `base-url` config key. The `JEVPIPE_BASE_URL` environment variable is removed.

**User config**

- **FR-010**: jevpipe MUST read an optional user config file in TOML from the platform's user config directory (`%APPDATA%\jevpipe\config.toml` on Windows; `$XDG_CONFIG_HOME/jevpipe/config.toml`, by default `~/.config/jevpipe/config.toml`, on Linux and macOS), or from the file named by `JEVPIPE_CONFIG` when set. There is no project-level config.
- **FR-011**: The config keys MUST be `model`, `concurrency`, `request-timeout`, `max-cost`, `max-time` (named and validated exactly like the flags of the same name) and `base-url`. A flag MUST override the config file, which MUST override the built-in default. `--threshold` and `--read-files` are not config keys.
- **FR-012**: An unknown key or an invalid value in the config file MUST fail runs, `config list` and `config get` before any input is read, naming the file, the key and (for unknown keys) the valid keys.
- **FR-013**: jevpipe MUST offer `config list` (every key with its effective value and origin, plus the API key's source without the key), `config get <key>` (the effective value only), `config set <key> <value>` (validated like the flag; creates the file and directory when missing; preserves other keys and comments), `config unset <key>` (accepts any key present in the file) and `config path` (the file in use, whether or not it exists). `config path`, `config set` and `config unset` MUST work while the file is broken.
- **FR-014**: The defaults shown by `--help` for config-backed options MUST be the effective ones.

**API key**

- **FR-015**: A run MUST use `OPENROUTER_API_KEY` when it is set and non-empty, without accessing the keychain; otherwise the key stored in the operating system's keychain; otherwise fail before reading input with a message naming both ways to provide a key.
- **FR-016**: `auth set-key` MUST read the key as one line from standard input and from nowhere else, trim surrounding whitespace, reject an empty key or one with other than printable ASCII characters or with spaces, quotes or backslashes (catching invisible characters picked up when pasting), and store it in the keychain, replacing any stored key. When standard input is a terminal it MUST show a prompt on standard error and hide the typed characters where the terminal allows it (saying so where it does not). The key MUST NOT be accepted as a command-line argument.
- **FR-017**: Interrupting the prompt (Ctrl+C) MUST restore the terminal to its previous state, store nothing, and exit with status 130.
- **FR-018**: `auth remove-key` MUST remove the stored key and succeed when none is stored.
- **FR-019**: A stored key MUST stay usable across jevpipe updates and rebuilds without any confirmation dialog on every platform.
- **FR-020**: Where no keychain is available, `auth set-key` and `auth remove-key` MUST fail with a message pointing to `OPENROUTER_API_KEY`, exit status 2.
- **FR-021**: The API key MUST never be written to standard output, standard error, the config file or any other file.

**Help and verification**

- **FR-022**: `--help` MUST describe the new options, the exit status 3, the config commands and keys, the config file location and the ways to provide the API key.
- **FR-023**: All behaviour MUST be verifiable by automated tests without network access, an API key, the real keychain or the developer's own config file. Commands MUST be tested as the real binary against the local stand-in service, with `JEVPIPE_CONFIG` pointing at a test file. Keychain access, which the binary cannot be pointed away from, MUST be tested in-process against a stand-in keychain, as the project's testing rule in CLAUDE.md allows.

### Key Entities

- **Run limits**: an optional spend limit in US dollars and an optional time limit, per invocation.
- **Stop**: why a run ended early (spend limit, time limit, key limit, credits), the amount spent, and the resume line.
- **User config**: the optional file with default values for the config keys; each effective value has an origin (flag, config file, built-in default).
- **Stored API key**: one OpenRouter key per user in the operating system's keychain; the environment variable takes precedence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a spend limit, the reported spend of a run never exceeds the limit by more than the cost of the requests in flight when it was reached, in 100% of test scenarios.
- **SC-002**: With a time limit T, the process exits within T plus 1 second, in 100% of test scenarios.
- **SC-003**: Every run stopped by a limit exits with status 3 and names the limit and the resume line; rerunning from that line and joining both outputs equals an uninterrupted run, in 100% of test scenarios.
- **SC-004**: A temporary in-flight budget refusal never stops a run.
- **SC-005**: After one `config set` per value, a user's runs need no flags for model, concurrency, timeouts or limits.
- **SC-006**: After `auth set-key` once, runs in a new shell need no environment variable.
- **SC-007**: The API key never appears in any output, file or command line, in 100% of test scenarios.
- **SC-008**: Reading a stored key adds less than 50 milliseconds to an invocation (measured: 1–4 ms on Windows, 7–10 ms on Linux, about 18 ms on macOS).
- **SC-009**: The full automated test suite passes on Linux, Windows and macOS without network, API key, keychain access or user config.

## Assumptions

- Cost comes from OpenRouter's per-answer cost report, which every Jev answer includes; the limit is enforced on reported cost, not on an estimate, so the overshoot is bounded by the requests in flight (at most `--concurrency`).
- The spend limit is a soft stop because cost is known only after an answer; the time limit is a hard stop because a caller that sets a deadline expects the process to end by then, like `timeout`. Exit status 3 follows the convention of a distinct status for "stopped by a limit you set" (for example `timeout` exits 124, curl 28); `filter` keeps grep's 0/1/2 and adds 3.
- OpenRouter distinguishes its "payment required" cases by a documented machine-readable field: the key's limit, the account's credits, and a temporary in-flight budget that comes with a wait time.
- The keychain is Windows Credential Manager, the macOS keychain, and the Secret Service on Linux desktops (GNOME Keyring, KWallet). Headless Linux, containers and CI usually have none; there the environment variable is the way, as today. The Linux kernel keyring is not used because it does not survive a reboot.
- On macOS, keychain entries are bound to the program that created them, and jevpipe is not signed with a stable identity: a spike showed an updated build blocking on its own stored key. The key is therefore stored and read through the system's own keychain tool, as the GitHub CLI does, so updates never trigger a dialog. The trade-off, accepted as for the GitHub CLI: other programs of the same user can read the key through that tool without a dialog; it is still encrypted at rest and locked with the keychain. The goal of storing the key is to prevent accidental leaks (plaintext files, shell history, logs, command lines), not to defend against a malicious program running as the same user.
- The key is read once per invocation and only when `OPENROUTER_API_KEY` is not set.
- Standard input is the only key source so that typing, pasting and piping behave the same and the command can never wait on a different device.
- Config keys use the flag names, so `config set request-timeout 20s` reads like `--request-timeout 20s`.
- Durations use the established systemd-style time span format (`500ms`, `90s`, `10m`, `1h30m`, `1h 30m`), parsed by a standard parser rather than a hand-written one; a unit is always required so a value is never ambiguous.
- Where both apply, "stopped by a limit" (exit status 3) outranks failed records (2), because an incomplete output matters most to the caller; failed records are still reported on standard error.
- `--help` shows the configured values as defaults, so the help text always tells the truth about what a run will use; `config list` shows where each value comes from.
- Removing `JEVPIPE_BASE_URL` and changing `--request-timeout` to a duration are breaking changes; nothing was released, so no compatibility period is needed.

## Out of Scope

- Budgets accumulated across invocations (the key's own spend limit covers that), token budgets, and cost estimates before a run.
- Other providers than OpenRouter, including TypeSafe's own endpoint.
- Project-level config files and environment variables per setting.
- Logging in through the browser (OpenRouter's OAuth flow) and checking a key against the service when it is stored.
- Caching identical requests.

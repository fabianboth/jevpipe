# Research: Run Limits, User Config and a Stored API Key

The earlier milestones' choices (runtime, HTTP client, ordered concurrency, retries, record reading,
questions and answers) stay as recorded in
[../001-filter-foundation/research.md](../001-filter-foundation/research.md) and
[../002-map-command/research.md](../002-map-command/research.md). This file covers what limits, the
config file and the stored key add. Prototypes live in the gitignored `scratch/` folder
(`keyproto`, `rtproto`, `echoproto`, `configproto`, `smallproto`); a throwaway branch ran the
keychain checks on GitHub's macOS and Linux runners on 2026-09-27.

## R1. OpenRouter's "payment required" (402) cases

Sources: OpenRouter docs `api_reference/limits` and `errors-and-debugging` (checked 2026-09-27).

| `error.metadata.limit_source` | Meaning | Handling |
|---|---|---|
| `openrouter_key_limit` | the API key's own credit limit is used up | stop: key limit (exit 3) |
| `openrouter_credits` | the account balance cannot cover the request | stop: credits (exit 3) |
| `openrouter_in_flight_budget` (`reason: in_flight_budget_exhausted`) | running and recently finished requests fill the in-flight spending budget; comes with `Retry-After` | transient: retried like 429, honouring `Retry-After` |
| missing (the System One example 402 has no `metadata`) | unknown | run-level error, exit 2, as today |

- **Decision**: parse `error.metadata.limit_source` from the error body and branch on it; never on
  `remedy_hint` or the message text (the docs say to branch on `limit_source`).
- **Rationale**: today `service.rs` turns every 402 into `Rejected`, so a temporary in-flight refusal
  ends the run with exit 2; at the default concurrency of 100 that is a realistic way to lose a run.
- **Also checked**: `usage.cost` is optional in the response schema, but the Jev guide says every Jev
  answer includes it; it stays `Option`, and FR-006 covers its absence under `--max-cost`.
  `GET /api/v1/key` returns `limit`, `limit_remaining`, `limit_reset` and usage; not used (no
  pre-flight calls).

## R2. Spend accounting

- **Decision**: count cost in whole nano-dollars (`u64`, 1e-9 USD): each reported `usage.cost` is
  rounded to nano-dollars when the reply is read, and `--max-cost` is parsed into the same unit. A
  shared meter (`AtomicU64`) adds each reply's cost as it arrives (out of order) and answers "may
  another request be sent?" (`spent < limit`). The summary's total comes from the same meter.
- **Rationale**: OpenRouter reports costs with up to 9 decimals (`0.000017766`); summing binary floats
  makes `10 × 0.0001 ≥ 0.001` false. Integer nano-dollars are exact for every reported value and need
  no crate.
- **Alternatives considered**: `f64` with an epsilon (fragile, surprising at the boundary);
  `rust_decimal` (a dependency for one addition and one comparison).

## R3. Stopping a run (soft spend stop, hard time stop, key limits)

- **Decision**:
  - One `Limits` object per run holds the spend, the deadline and the stop reason (first wins). A
    stop is set when a request is refused before sending (spend limit reached) or by the service
    (key limit, credits). The record gets no outcome ("unprocessed") and the input stream ends at
    the stop (`take_while` on the input), so no new records enter; records already in flight
    finish and their cost is counted, but output stops at the first unprocessed record.
  - The spend check happens right before a request is sent, so records needing no request (skipped
    files, invalid lines) before the stop point are still printed.
  - The time limit is a `select!` between the next decision and a deadline (`sleep_until(start +
    max_time)`) in the pipeline loop. At the deadline the loop breaks, the in-flight futures are
    dropped (abandoning their requests).
  - The resume line is the same for every trigger: the line after the last decided record (1 when
    none was). Only blank lines can lie between it and the first unprocessed record, and blank
    lines are not records, so rerunning from either line gives the same result.
  - A stopped run prints `jevpipe: stopped: …; input from line L on was not processed` before the
    summary and exits 3. Exit precedence: output closed → 0; stopped → 3; failed records or a
    run-level error → 2; otherwise as before.
- **Rationale**: the soft stop keeps every paid answer that can be printed in order; ending the input
  at the signal keeps a stdin that never closes (a step-loop driver) from being read forever. The
  hard stop meets a deadline like `timeout` does.
- **Exit without waiting**: input is read on a plain OS thread (`record.rs`), not a tokio blocking
  task, so returning from `main` ends the process even while that thread is blocked on stdin. New
  code that blocks on input MUST follow the same pattern: `tokio::task::spawn_blocking` would make
  the runtime's shutdown wait for the read (the first Ctrl+C prototype hung until Enter because of
  exactly this).
- **Alternatives considered**: stopping at the first unprocessed record without draining (loses the
  cost of in-flight answers from the report); estimating cost before sending (no tokenizer, see the
  first milestone).

## R4. Durations

- **Decision**: `humantime` 2.4 (`parse_duration`), wrapped in our value parser that rejects zero and
  rewords errors ("`10` needs a unit, for example 10s or 5m"). Display with `humantime::format_duration`.
  `--request-timeout` default becomes `10s`.
- **Rationale**: the de facto Rust crate for systemd-style time spans (`500ms`, `90s`, `10m`, `1h30m`,
  `1h 30m`); no dependencies, MIT OR Apache-2.0, maintained again under the chronotope org (2.4.0 on
  2026-07-02; the 2025 "unmaintained" advisory was withdrawn). A bare number is rejected by the crate.
- **Alternatives considered**: `jiff` friendly durations (bigger, longer error messages, rejects days);
  `fundu`, `duration-str` (niche); hand-rolled parsing (ruled out).

## R5. Config file: layering under clap

Prototype: `scratch/configproto` (clippy- and deny-clean with the repo's lints).

- **Decision**: load the config file before parsing the command line, then set its values as clap
  defaults on the `filter` and `map` subcommands (`Command::mut_arg(id, |arg| arg.default_value(v))`,
  clap feature `string`), and parse with `FromArgMatches`. The config-backed flags live in one
  flattened `Settings` args struct, so names, parsers and built-in defaults are defined once; config
  keys are the flags' long names.
  - Validation reuse: a config value is checked by parsing `--<key>=<value>` with a command built from
    `Settings::augment_args` (clap's `ValueParser::parse_ref` is not public). The same check backs
    `config set`.
  - Origin for `config list`: the config file when the key is set there, otherwise the built-in
    default (`Arg::get_default_values` on the unmodified command).
  - `base-url` is a config-only key (FR-009): not a clap argument, parsed with `reqwest::Url` and
    limited to `http`/`https`.
  - A broken config file fails `filter`, `map`, `config list` and `config get` before any input is
    read; `config path`, `config set` and `config unset` work on the raw document; `--help` falls back
    to the built-in defaults.
  - `mut_arg` panics on an unknown id: ids come only from `Settings`, never from the file, so the key
    check runs before any `mut_arg`.
- **Rationale**: `--help` shows the effective default (FR-014) and every key has one definition.
- **Alternatives considered**: `Option` fields merged with a serde struct (each key in four places,
  `--help` wrong); `figment` (last release 2024, `toml` 0.8), `config` (heavy, no origins), `confique`,
  `clap-serde-derive`, `twelf` (stale or no clap tie-in).

## R6. Config file: location and editing

- **Decision**: `etcetera` 0.11 `choose_base_strategy()` for the directory: `%APPDATA%` on Windows,
  `$XDG_CONFIG_HOME` or `~/.config` on Linux and macOS; `JEVPIPE_CONFIG` overrides the file.
  `toml_edit` 0.25 (`DocumentMut`) reads and edits the file: `set` keeps the old value's decor
  (comments, alignment), `unset` removes the key. `set` writes an integer, a float or a string,
  whichever the value parses as first.
- **Rationale**: `dirs`/`directories` put macOS config under `~/Library/Application Support`; CLI
  users expect `~/.config`, as with `gh` and `git`. `toml_edit` preserves formatting, and the separate
  `toml` crate is not needed.
- **Known effects**: `unset` also drops the comment lines directly above the key; a rewritten string
  switches to double quotes.

## R7. Keychain crates and platforms

Measured with release builds (key read once per invocation, including opening the store):

| Platform | Backend | Read | Extra crates |
|---|---|---|---|
| Windows | `windows-native-keyring-store` 1.1 (Credential Manager), `default-features = false` (drops `regex`) | 1–4 ms | 7 |
| Linux | `zbus-secret-service-keyring-store` 1.0 (Secret Service), `default-features = false`, feature `rt-async-io-crypto-rust` | 7–10 ms | 73 |
| macOS | our store over `/usr/bin/security` (R8) | ~18 ms | 0 |

All behind `keyring-core` 1.0 (`Entry`, `set_default_store`), MIT OR Apache-2.0.

- **Linux runtime feature**: with `rt-tokio-*`, a keychain call made from async code panics ("Cannot
  start a runtime from within a runtime"); with `rt-async-io-*` it works from async code,
  `spawn_blocking` and a plain thread. The store crate enables zbus's default `async-io` runtime
  anyway, so choosing it adds nothing (195 crates on Linux either way, 122 without the keychain).
- **Linux without a Secret Service** (headless, WSL, containers): opening the store fails at once with
  `org.freedesktop.secrets was not provided`; no hang, no `libdbus` needed (pure Rust, works in a
  static musl build). So the keychain is only opened when `OPENROUTER_API_KEY` is unset.
- **Linux kernel keyring** (`linux-keyutils-keyring-store`): ruled out, in memory only, lost on reboot.
- **`cargo deny`**: all licenses allowed; duplicates `syn` 2/3 (Linux) and nothing new elsewhere, warn
  only.
- **Entry**: service `jevpipe`, user `openrouter-api-key`.
- **Errors**: `keyring_core::Error` is `#[non_exhaustive]`; match `NoEntry` with `matches!` and treat
  everything else as "keychain unavailable or denied" with the store's message.

## R8. macOS: the system keychain tool instead of the keychain API

- **Finding** (macOS runner): an item written through the keychain API by one build is bound to that
  build's ad-hoc signature. A rebuilt binary blocked on reading and on replacing the item (killed after
  60 s) and could not delete it ("Invalid attempt to change the owner of this item", -25244).
  Rust binaries are ad-hoc signed, so every update would do this, or show a dialog on a desktop.
- **Decision**: on macOS, store and read the key through `/usr/bin/security`, as the GitHub CLI does
  via `zalando/go-keyring`:
  - write: `security -i` with `add-generic-password -U -s jevpipe -a openrouter-api-key -w "<key>"`
    on **stdin** (never in argv);
  - read: `security find-generic-password -s jevpipe -a openrouter-api-key -w` (key on stdout);
  - remove: `security delete-generic-password -s jevpipe -a openrouter-api-key`;
  - exit status 44 means "not found".
  Verified on the runner: set, overwrite, read and delete work without any dialog; the item's access
  list trusts only `/usr/bin/security` (`identifier "com.apple.security" and anchor apple`), which
  does not change with jevpipe updates.
- **Shape**: implemented as a `keyring-core` credential store (`CredentialStoreApi` + `CredentialApi`,
  about ten small methods), so every platform goes through `keyring_core::Entry` and the mock store
  covers the lookup logic everywhere.
- **Key characters**: because the key is written inside a `security -i` command line, and to keep
  one rule on every platform, `auth set-key` accepts only printable ASCII characters without spaces
  without quotes or backslashes (OpenRouter keys are `sk-or-v1-` plus hex).
- **Trade-off (accepted with the user)**: other programs of the same user can read the item through
  `security` without a dialog; the goal is preventing accidental leaks, not malicious local programs.
- **Alternatives considered**: `apple-native-keyring-store` (breaks on update, above); signing with a
  Developer ID (needs an Apple account and does not help `cargo install` users); disabling keychain
  user interaction (turns the dialog into an error the user cannot fix without Keychain Access).

## R9. Reading the key: hidden input on standard input

- **Decision**: our own `prompt` module (~40 lines, no `unsafe`): switch off echo **on stdin itself**,
  read one line, restore the exact previous state in `Drop`.
  - Windows: `winapi-util` `console::mode` / `set_mode` on `HandleRef::stdin()`, clearing
    `ENABLE_ECHO_INPUT` (0x0004).
  - Unix: `rustix` `termios::tcgetattr` / `tcsetattr` on `stdio::stdin()`, removing `LocalModes::ECHO`.
  - If switching off echo fails, stdin is not a console (a pipe, or Git Bash's mintty without
    pseudo console support): read the line as is. The prompt is shown when `std::io::IsTerminal` says
    stdin is a terminal (true for mintty, false for pipes), and says the input is visible when echo
    could not be switched off.
  - Ctrl+C: the line is read on a plain thread (R3) while `tokio::signal::ctrl_c()` is awaited; on
    Ctrl+C the guard restores the terminal, `interrupted` is printed and the command returns exit 130
    without waiting for the read. Needs tokio's `signal` feature.
- **Verified**: Windows Terminal/VS Code, PowerShell and cmd hide the input; Git Bash's mintty
  (`MSYS=disable_pcon`) reads it visibly without hanging; a Linux pseudo-terminal (WSL `script`)
  hides it and restores echo; Ctrl+C restores echo and exits 130 at once on Windows and Linux. Without
  the Ctrl+C handling, Unix leaves the terminal silent (PowerShell happens to reset it itself).
- **Rationale**: stdin is the only key source (typing, pasting and piping behave the same). Go's
  `golang.org/x/term.ReadPassword(stdin)` works the same way; Python's `getpass` restores in `finally`.
- **Alternatives considered**: `rpassword` and `console` (open `/dev/tty` or `CONIN$`, which hangs in
  mintty; `rpassword` implements its own line editor); `passterm` (does this, but small, own unsafe FFI,
  forces echo on instead of restoring, errors on pipes); raw mode with our own line editor (as
  `rpassword` 7.5: ~150 lines of editing keys).

## R10. Testing without the network, the real keychain or the user's config

- **Config**: every binary test sets `JEVPIPE_CONFIG` to a file in a temp dir; the stand-in's address
  goes in as `base-url` (replacing `JEVPIPE_BASE_URL`). A test helper writes the file.
- **Keychain**: the lookup order and `auth` logic are tested in-process against
  `keyring_core::mock::Store`. The default store is process-global, so these tests share one
  `#[test]` (or a lock) instead of running in parallel. Binary tests only cover paths that do not
  reach a keychain (empty key, key as an argument, `OPENROUTER_API_KEY` set). The macOS `security`
  store and the platform stores are checked manually (quickstart), as the spike did.
- **Hidden prompt**: echo control needs a real terminal; the piped path is covered by binary tests,
  the terminal path by the quickstart (and the spike above).
- **Limits**: the stand-in reports a configurable cost per answer, can delay answers and can answer
  402 with each `limit_source` (markers in the record, like the existing `status:` and `slow:`).

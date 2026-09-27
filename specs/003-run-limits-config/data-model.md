# Data Model: Run Limits, User Config and a Stored API Key

Extends [../002-map-command/data-model.md](../002-map-command/data-model.md) (Record, Input,
Questions, Answers, Outcome, Decision, Summary). Only new or changed types are listed.

## Settings (config-backed run options)

The run options that a config file can default, as one flattened clap args struct shared by `filter`
and `map` (`RunArgs` keeps `--read-files` and flattens `Settings`).

| Key / flag | Type | Built-in default | Rule |
|---|---|---|---|
| `model` | string | `~typesafe/jev-latest` | non-empty |
| `concurrency` | positive integer | `100` | ≥ 1 |
| `request-timeout` | Duration | `10s` | > 0, unit required |
| `max-cost` | Limit of Cost | `none` | `none` or a positive amount of US dollars |
| `max-time` | Limit of Duration | `none` | `none` or a positive duration, unit required |

The key is the flag's long name; the value is validated by the flag's own parser, wherever it comes
from (command line, config file, `config set`).

## Cost

An amount of US dollars as whole nano-dollars (`u64`, 1 nano-dollar = 1e-9 USD).

- From the service: `usage.cost` (a JSON number) rounded to the nearest nano-dollar.
- From `--max-cost`: a decimal number of dollars (`0.5`, `0.50`, `2`), > 0, at most 9 decimals.
- Displayed as dollars with up to 6 decimals, trailing zeros trimmed (`$0.001`, `$0.51`), as the
  summary does today.

## Duration

A `std::time::Duration` parsed from a systemd-style span with a unit (`500ms`, `90s`, `10m`, `1h30m`,
`1h 30m`), > 0. Displayed in the same format (`10s`, `1h 30m`).

## Limit

`none` or a value (`Cost` or `Duration`). "No limit" is a real value here: a config file can set a
limit and `--max-cost none` / `--max-time none` lifts it for one run.

## Limits (one per run)

Shared by all in-flight requests of one run; the pipeline's only view of limits.

| Field | Meaning |
|---|---|
| `spent` | nano-dollars reported so far (atomic; added as each reply arrives) |
| `max_cost` | the run's `max-cost` Limit |
| `deadline` | start of the run + `max-time`, when set |
| `stop` | the Stop reason, set once (first wins) |

- `may_send()`: no stop yet, and `max_cost` is `none` or `spent < max_cost`; when false because of
  the spend, it sets the stop to `SpendLimit`.
- `add(cost)`: adds a reply's cost; with a `max_cost`, a reply without a cost is a run-level error
  (FR-006).
- `stop(reason)`: sets the stop (402 key limit or credits, the deadline).
- `stopped()`: whether a stop is set (ends the input stream).
- The summary's cost is `spent` at the end of the run, so answers drained after a stop are included.

## Stop

Why a run ended early; the first reason wins.

| Reason | Set when | Message part |
|---|---|---|
| `SpendLimit { limit, spent }` | a record is about to be sent and `may_send()` is false | `spend limit $0.50 reached ($0.51 spent)` |
| `TimeLimit { limit }` | the deadline `start + max-time` passes | `time limit 10m reached` |
| `KeyLimit` | the service answers 402 with `openrouter_key_limit` | `the API key's spend limit is used up` |
| `Credits` | the service answers 402 with `openrouter_credits` | `the OpenRouter account's credits are used up` |

Plus the **resume line** `L`, the same for every reason: the line after the last decided record (1
when none was). Only blank lines can lie between it and the first unprocessed record, so rerunning
from either gives the same result.

Standard error: `jevpipe: stopped: <message part>; input from line <L> on was not processed`,
before the summary line. A stop is only reported when a record was left unprocessed; a limit reached
after the last record ends the run normally.

## Decided (pipeline, changed)

What the pipeline gets back per input, in input order:

| Variant | Meaning | Output |
|---|---|---|
| `Record(Decision)` | answered, skipped or failed (as before) | printed, counted |
| `Input(FailedInput)` | an input file that could not be read (as before) | reported by name |
| `Unprocessed` | not sent because of a stop | not printed; ends the output |

After the first `Unprocessed`, the input stream ends (no new records are read), in-flight requests
finish, and nothing more is printed.

## Exit (changed)

| Exit | Status | When |
|---|---|---|
| `Success` | 0 | as before, or the output consumer went away |
| `NothingKept` | 1 | `filter`: nothing kept, nothing failed, no stop |
| `Error` | 2 | a record failed, a run-level error, a usage or config error, no keychain |
| `Stopped` | 3 | a limit stopped the run |
| `Interrupted` | 130 | Ctrl+C at the `auth set-key` prompt |

Precedence: output closed → 0; stopped → 3; failed or run-level error → 2; then the command's rule.

## Service errors (changed)

| Class | From | Handling |
|---|---|---|
| `Transient { retry_after }` | 408, 429, 5xx, 524, 529, timeouts, **402 `openrouter_in_flight_budget`** | retried |
| `TooLarge` | 413, 400 `max_tokens_exceeded` | record fails |
| `Limit(KeyLimit \| Credits)` | **402 with `openrouter_key_limit` / `openrouter_credits`** | stop (exit 3) |
| `Rejected(message)` | anything else, including 402 without `limit_source` | run-level error (exit 2) |

## Config file

Optional TOML file, one table of keys, found at `JEVPIPE_CONFIG` or the platform's user config
directory (`%APPDATA%\jevpipe\config.toml`, `$XDG_CONFIG_HOME/jevpipe/config.toml`,
`~/.config/jevpipe/config.toml`).

| Key | Type in the file | Validated as |
|---|---|---|
| `model`, `request-timeout`, `max-time` | string | the flag |
| `concurrency` | integer | the flag |
| `max-cost` | float, integer or `"none"` | the flag |
| `base-url` | string | an absolute `http` or `https` URL; default `https://openrouter.ai/api` |

- A missing file is an empty config.
- Any scalar is accepted in the file and validated as text by the flag's parser.
- An unknown key, a non-scalar value or an invalid value is a config error naming the file and the
  key (and, for unknown keys, the valid keys).

Each effective value has an **origin**: `config file` or `default` (flags override both at run time
but are not shown by `config list`).

## API key

| Source | When used |
|---|---|
| environment | `OPENROUTER_API_KEY` is set and non-empty; the keychain is not opened |
| keychain | otherwise, when the keychain holds an entry (service `jevpipe`, user `openrouter-api-key`) |
| none | otherwise: error naming `OPENROUTER_API_KEY` and `jevpipe auth set-key` |

Stored key rules (`auth set-key`): one line from standard input, trimmed, non-empty, and only printable
ASCII characters without spaces, quotes (`"`, `'`) or backslashes (`\`), which also catches
invisible characters picked up when pasting (zero-width or non-breaking spaces, control characters,
terminal paste markers). The input is read and checked before the keychain
is opened, so an invalid key never touches it. The key is never printed, logged or written to a file.

## Keychain

`keyring_core::Entry` over one default store chosen at startup:

| Platform | Store |
|---|---|
| Windows | Credential Manager (`windows-native-keyring-store`) |
| Linux | Secret Service (`zbus-secret-service-keyring-store`, async-io runtime) |
| macOS | our store over `/usr/bin/security` (secret on stdin; exit 44 = not found) |
| tests | `keyring_core::mock::Store` |

Results: key, no entry, or unavailable (no Secret Service, locked, denied) with the store's message.

# CLI Contract: limits, `config` and `auth`

Extends [../../002-map-command/contracts/cli.md](../../002-map-command/contracts/cli.md) (records,
questions, standard output formats); the sections below replace its Options, Environment, Standard
error and Exit status sections.

## Synopsis

```text
jevpipe filter [OPTIONS] <QUESTION> [FILES]...
jevpipe map [OPTIONS] <--questions <JSON>|--questions-file <FILE>> [FILES]...
jevpipe config list
jevpipe config get <KEY>
jevpipe config set <KEY> <VALUE>
jevpipe config unset <KEY>
jevpipe config path
jevpipe auth set-key
jevpipe auth remove-key
```

## Run options (`filter` and `map`)

| Option | Commands | Default | Config key | Meaning |
|---|---|---|---|---|
| `-q, --questions <JSON>` | `map` | — | — | as in 002 |
| `-f, --questions-file <FILE>` | `map` | — | — | as in 002 |
| `--read-files` | both | off | — | as in 002 |
| `--threshold <P>` | `filter` | `0.5` | — | as in 002 |
| `--concurrency <N>` | both | `100` | `concurrency` | maximum requests in flight; N ≥ 1 |
| `--model <ID>` | both | `~typesafe/jev-latest` | `model` | model to ask |
| `--request-timeout <DURATION>` | both | `10s` | `request-timeout` | abandon a request after this long and retry it |
| `--max-cost <DOLLARS\|none>` | both | `none` | `max-cost` | stop sending once the reported cost of this run reaches this many US dollars |
| `--max-time <DURATION\|none>` | both | `none` | `max-time` | stop the run once it has taken this long |

`DURATION`: a positive span with a unit: `500ms`, `90s`, `10m`, `1h30m`, `1h 30m`. A bare number is a
usage error:

```text
error: invalid value '10' for '--request-timeout <DURATION>': needs a unit, for example 10s or 5m
```

`DOLLARS`: a positive decimal number (`0.5`, `2`). `none` lifts a limit set in the config file.

The `[default: …]` shown by `--help` is the effective default: the config file's value when set,
otherwise the built-in one. With a broken config file, `--help` shows the built-in defaults.

## Environment

| Variable | Meaning |
|---|---|
| `OPENROUTER_API_KEY` | the API key; when set and non-empty it wins and the keychain is not opened |
| `JEVPIPE_CONFIG` | path of the config file to use instead of the default location |

`JEVPIPE_BASE_URL` is removed; the service address is the `base-url` config key.

## Config file

TOML, keys as in the table above plus `base-url`:

```toml
# ~/.config/jevpipe/config.toml
model = "typesafe/jev-1.13"
max-cost = 1.0
request-timeout = "20s"
base-url = "https://openrouter.ai/api"   # jevpipe appends /v1/systemone
```

Default location: `%APPDATA%\jevpipe\config.toml` (Windows), `$XDG_CONFIG_HOME/jevpipe/config.toml`
or `~/.config/jevpipe/config.toml` (Linux, macOS). A missing file is an empty config.

Config errors (exit 2, before any input is read, for `filter`, `map`, `config list`, `config get`):

```text
jevpipe: error: C:\Users\me\AppData\Roaming\jevpipe\config.toml: unknown key `concurency`; the keys are base-url, concurrency, max-cost, max-time, model, request-timeout
jevpipe: error: /home/me/.config/jevpipe/config.toml: invalid value '0' for `concurrency`: number would be zero for non-zero type
jevpipe: error: /home/me/.config/jevpipe/config.toml: TOML parse error at line 2, column 7 …
```

## `config` commands

| Command | Output (stdout) | Notes |
|---|---|---|
| `config list` | every key with its effective value and origin, then the API key's source | fails on a broken file |
| `config get <KEY>` | the effective value alone, e.g. `20s`, `none`, `100` | fails on a broken file or an unknown key |
| `config set <KEY> <VALUE>` | nothing | validated like the flag; creates the file and its directory; keeps other keys and comments; works on a broken file |
| `config unset <KEY>` | nothing | removes the key; accepts any key present in the file, even an unknown one; works on a broken file |
| `config path` | the path of the file in use | whether or not it exists |

`config list` prints valid TOML with the origin as a comment, so it can be copied into the file:

```text
base-url = "https://openrouter.ai/api"  # default
concurrency = 100                       # default
max-cost = 1.0                          # config file
max-time = "none"                       # default
model = "typesafe/jev-1.13"             # config file
request-timeout = "20s"                 # config file
# API key: from the keychain
```

The API key line is one of `from OPENROUTER_API_KEY`, `from the keychain`, `not set`, or
`keychain unavailable: <reason>`; it never shows the key.

Rejected values name the key: `jevpipe: error: invalid value '0' for \`concurrency\`: …` (exit 2,
file unchanged).

## `auth` commands

| Command | Behaviour | Exit |
|---|---|---|
| `auth set-key` | reads one line from standard input (the key), stores it in the keychain, replacing a stored key; prints `jevpipe: API key stored in the keychain` to stderr | 0; 2 on an empty or invalid key or no keychain; 130 on Ctrl+C |
| `auth remove-key` | removes the stored key; `jevpipe: no API key stored` when there is none | 0; 2 when no keychain is available |

`auth set-key` takes no key argument. When standard input is a terminal, it prints
`OpenRouter API key: ` to stderr and hides the typing; where the terminal cannot hide it, the prompt
is `OpenRouter API key (input will be visible): `. When standard input is a pipe, no prompt:

```sh
echo "$OPENROUTER_API_KEY" | jevpipe auth set-key
```

Errors:

```text
jevpipe: error: no API key given
jevpipe: error: that does not look like an API key (printable ASCII characters only, no spaces, quotes or backslashes)
jevpipe: error: no keychain available here (org.freedesktop.secrets was not provided by any .service files); set OPENROUTER_API_KEY instead
```

## Standard error (runs)

As in 002, plus the stop line before the summary when a limit stopped the run:

```text
jevpipe: line 4: not found
jevpipe: stopped: spend limit $0.5 reached ($0.5012 spent); input from line 812 on was not processed
jevpipe: 811 records, 809 answered, 1 skipped, 1 failed, $0.5012, 41.3s
```

Stop messages:

| Reason | Line |
|---|---|
| spend limit | `stopped: spend limit $<limit> reached ($<spent> spent); input from line L on was not processed` |
| time limit | `stopped: time limit <duration> reached; input from line L on was not processed` |
| key limit | `stopped: the API key's spend limit is used up; input from line L on was not processed` |
| credits | `stopped: the OpenRouter account's credits are used up; input from line L on was not processed` |

A missing key: `jevpipe: error: no API key: set OPENROUTER_API_KEY or run jevpipe auth set-key`.
With `--max-cost` and an answer without a cost:
`jevpipe: error: the service reported no cost, so --max-cost cannot be enforced`.

## Exit status

| Status | `filter` | `map` | `config`, `auth` |
|---|---|---|---|
| 0 | kept at least one, none failed, not stopped; or the output consumer went away | none failed, not stopped; or the output consumer went away | success |
| 1 | nothing kept, none failed, not stopped | not used | not used |
| 2 | a record or input failed, a run-level error, a usage or config error | same | usage, config or keychain error, invalid key |
| 3 | a limit stopped the run (spend, time, key limit, credits) | same | not used |
| 130 | not used | not used | `auth set-key` interrupted with Ctrl+C |

Precedence for runs: output consumer gone → 0; stopped → 3; failed or run-level error → 2; then the
command's own rule.

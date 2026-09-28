# CLI Contract: providers, `--max-tokens` and the summary (changes)

Extends [../../003-run-limits-config/contracts/cli.md](../../003-run-limits-config/contracts/cli.md).
Only what changes is listed.

## Run options (`filter` and `map`)

| Option | Default | Config key | Meaning |
|---|---|---|---|
| `--model <ID>` | `jev-latest` (was `~typesafe/jev-latest`) | `model` | model to ask; pin a version such as `jev-1.13` (OpenRouter) or `jev-1.13.0` (TypeSafe) |
| `--max-tokens <COUNT\|none>` (new) | `none` | `max-tokens` | stop sending requests once this run's reported tokens reach COUNT, e.g. `250k` or `5M` |

`COUNT`: a positive number with an optional `k` or `M` suffix (`250k`, `5M`, `1.5M`, `1000000`).

```text
error: invalid value '0' for '--max-tokens <COUNT|none>': must be positive
error: invalid value 'lots' for '--max-tokens <COUNT|none>': not a count, for example 250k or 5M
```

## Environment

| Variable | Meaning |
|---|---|
| `OPENROUTER_API_KEY` | the key when the provider is `openrouter`; when set and non-empty the keychain is not opened |
| `TYPESAFE_API_KEY` (new) | the key when the provider is `typesafe`; same rule |
| `JEVPIPE_CONFIG` | unchanged |

## Config file

New keys `provider` (`openrouter` or `typesafe`, config only) and `max-tokens`:

```toml
provider = "typesafe"
max-tokens = "5M"
```

`config list` (provider TypeSafe, key from the environment):

```text
base-url = "https://api.typesafe.ai"    # default
concurrency = 100                       # default
max-cost = "none"                       # default
max-time = "none"                       # default
max-tokens = "5M"                       # config file
model = "jev-latest"                    # default
provider = "typesafe"                   # config file
request-timeout = "10s"                 # default
# API key: from TYPESAFE_API_KEY
```

An invalid provider:
`jevpipe: error: invalid value 'anthropic' for \`provider\`: the providers are openrouter, typesafe`.

## Provider sections

The config file may hold a table per provider; its keys apply only while that provider is active
and beat the top level. A flag beats both.

```toml
provider = "openrouter"
concurrency = 50                 # every provider

[openrouter]
max-cost = 0.5
model = "jev-1.13"

[typesafe]
max-tokens = "5M"
model = "jev-1.13.0"
```

| Command | Effect |
|---|---|
| `config set openrouter.max-cost 0.5` | writes `max-cost = 0.5` under `[openrouter]`, creating the table |
| `config unset typesafe.model` | removes it; removes `[typesafe]` when it becomes empty |
| `config get openrouter.model` | `jev-1.13`: the value that applies with OpenRouter active |
| `config get model` | the value for the active provider |

`config list` shows the active provider's values with their origin:

```text
max-cost = 0.5                          # config file [openrouter]
model = "jev-1.13"                      # config file [openrouter]
concurrency = 50                        # config file
```

Rejected, exit 2, file unchanged:

```text
jevpipe: error: invalid value '1' for `typesafe.max-cost`: TypeSafe reports no cost; use typesafe.max-tokens
jevpipe: error: unknown key `openrouter.provider`; the keys are base-url, concurrency, …, provider (top level only), each also as openrouter.<key> or typesafe.<key>
```

## `auth` commands

Act on the configured provider's key.

| Provider | Prompt | Confirmation |
|---|---|---|
| `openrouter` | `OpenRouter API key: ` | `jevpipe: OpenRouter API key stored in the keychain` |
| `typesafe` | `TypeSafe API key: ` | `jevpipe: TypeSafe API key stored in the keychain` |

`auth remove-key` with none stored: `jevpipe: no TypeSafe API key stored`. With a broken config
file, both fail naming the file (exit 2).

## Standard error (runs)

Summary with a provider that reports tokens but no cost:

```text
jevpipe: 811 records, 809 answered, 1 skipped, 1 failed, 290.4k tokens, 41.3s
```

With cost reported too (OpenRouter), both: `…, 1 failed, 290.4k tokens, $0.0122, 41.3s`.

New stop line:

| Reason | Line |
|---|---|
| token limit | `stopped: token limit 5000000 reached (5000312 used); input from line L on was not processed` |

New run-level errors (exit 2, before any input is read unless noted):

```text
jevpipe: error: no TypeSafe API key: set TYPESAFE_API_KEY or run jevpipe auth set-key (provider typesafe; jevpipe config set provider openrouter switches)
jevpipe: error: no OpenRouter API key: set OPENROUTER_API_KEY or run jevpipe auth set-key (provider openrouter; jevpipe config set provider typesafe switches)
jevpipe: error: TypeSafe reports no cost, so --max-cost cannot be enforced; use --max-tokens instead
jevpipe: error: max-cost = 0.5 in the config file applies to every provider, but TypeSafe reports no cost; keep it for OpenRouter with jevpipe config set openrouter.max-cost 0.5 and jevpipe config unset max-cost, and limit TypeSafe runs with max-tokens
jevpipe: error: the service reported no token count, so --max-tokens cannot be enforced
jevpipe: error: service error: 400 Bad Request: Unknown model: jev-1.13
```

The no-token-count error comes at the first answer that lacks one, as with `--max-cost`. The
missing-key message changes from `no API key: …` to name the provider.

## Exit status

Unchanged; a token-limit stop is exit 3 like the other limits.

## Help

- `filter --help`, `map --help`: `--max-tokens`, the new `--model` text, exit status 3 lists the
  token limit.
- `config --help`: `provider` and its values, that `base-url` defaults to the provider's address,
  and the `<provider>.<key>` form for provider sections.
- `auth --help`: both variables, and that the commands act on the configured provider's key.

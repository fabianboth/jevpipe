# Data Model: TypeSafe's Own API as a Second Provider

Extends [../003-run-limits-config/data-model.md](../003-run-limits-config/data-model.md). Only new
or changed entities are listed.

## Provider (new, `src/provider.rs`)

The service jevpipe talks to. A closed enum, matched exhaustively.

| Variant | Config value | Name in messages | Default base URL | Key variable | Keychain user | Reports cost |
|---|---|---|---|---|---|---|
| `OpenRouter` | `openrouter` (default) | OpenRouter | `https://openrouter.ai/api` | `OPENROUTER_API_KEY` | `openrouter-api-key` | yes |
| `TypeSafe` | `typesafe` | TypeSafe | `https://api.typesafe.ai` | `TYPESAFE_API_KEY` | `typesafe-api-key` | no |

- Parsed from the config key `provider`; any other value is a `KeyError::Invalid` naming the valid
  values.
- `other()` gives the other provider, for the switch command in the missing-key message.
- "Reports cost" gates the spend limit at run start (R6).

## Config (changed, `src/config/`)

- New key `provider` (config-only, like `base-url`, top level only); `keys::all()` lists it.
- `Scope` (new): `Every` (top level) or `Only(Provider)` (that provider's table).
- `ScopedKey` (new): a key with its scope, parsed from `max-cost` or `openrouter.max-cost`; the
  prefix must be a provider, the key a config key; `provider` is only valid unscoped and
  `max-cost` is not valid with `Only(TypeSafe)`.
- `Config { entries: Vec<(ScopedKey, String)>, provider: Provider }` (was a flat `values` list):
  every validated entry of the file.
- `Config::provider()` → the top-level `provider`, or `OpenRouter`.
- `Config::view(provider)` → the merged settings for that provider: its table's entries over the
  top level. `settings()`, `setting(key)` and `base_url()` read the active provider's view, so
  their callers are unchanged; `base_url()` falls back to the provider's address.
- `Origin`: `ConfigFile(Scope)` (was `ConfigFile`) or `Default`; shown as `config file`,
  `config file [typesafe]`, `default`.
- `Config::shared_spend_limit()` → the top-level `max-cost`, for the refusal message.

## Settings (changed, `src/settings.rs`)

| Field | Flag / key | Type | Default |
|---|---|---|---|
| `model` | `--model` / `model` | `String` | `jev-latest` (was `~typesafe/jev-latest`) |
| `max_tokens` (new) | `--max-tokens` / `max-tokens` | `Limit<Tokens>` | `none` |

`--max-tokens` values: a positive count parsed by `parse-size` (`250k`, `5M`, `1.5M`, `1000000`)
or `none`; zero is rejected (`must be positive`).

## Tokens (new, `src/tokens.rs`)

A count of tokens (`u64` newtype).

- `FromStr` via `parse-size`, rejecting zero where used as a limit.
- `Display` exact (`3300`), for the stop line and `config get`.
- `abbreviated()` via `unit-prefix` (`812`, `41.3k`, `12.5M`), for the summary.

## Usage (new, in `src/service/mod.rs`)

What one answer consumed, read from `usage`.

| Field | Source | Notes |
|---|---|---|
| `tokens` | `input_tokens + output_tokens` | `Option<Tokens>`: `None` when either is missing |
| `cost` | `cost` | `Option<Cost>`: OpenRouter only |

`Reply { answers, usage: Usage }` replaces `Reply { answers, cost }`. A malformed count (not a
non-negative integer) is an unexpected-answer error, like a malformed cost.

## ApiKey lookup (changed, `src/auth/`)

- `api_key(provider)`: `provider.key_variable()` when set and non-empty, else the keychain entry
  `jevpipe` / `provider.keychain_user()`, else `LookupError::Missing(provider)`, whose message names
  the provider, its variable, `auth set-key`, and `config set provider <other>`.
- `source(provider)` for `config list`: `from TYPESAFE_API_KEY`, `from the keychain`, `not set`,
  `keychain unavailable: …`.
- `auth set-key` / `remove-key` take the provider from the config; the prompt reads
  `TypeSafe API key: ` or `OpenRouter API key: `; the confirmation names the provider.

## Meter (new, in `src/limits.rs`)

One measure of a run against its limit; used for spend and for tokens.

| Field | Type | Meaning |
|---|---|---|
| `total` | `AtomicU64` | the sum reported so far (nano-dollars or tokens) |
| `reported` | `AtomicBool` | whether any answer reported this measure |
| `limit` | `Limit<u64>` | from settings |

- `add(Option<u64>) -> Result<(), Unreported>`: adds; `Err` when the limit is set and the value is
  missing.
- `reached() -> bool`, `reported() -> Option<u64>`.

`Measure` (`Spend` | `Tokens`) names a meter in messages: the limit's flag, how amounts are shown
(`$0.5` / `5000000`).

## Limits (changed, `src/limits.rs`)

- Fields: `spend: Meter`, `tokens: Meter` (replacing `spent`, `costed`, `max_cost`), the deadline,
  the stop.
- `add(usage)`: feeds both meters; `Err(Unreported(measure))` replaces `NoCost`:
  `the service reported no cost, so --max-cost cannot be enforced` /
  `the service reported no token count, so --max-tokens cannot be enforced`.
- `may_send()`: stops with `Stop::Limit(measure)` for the first meter reached.
- `reported()` → `Reported { cost: Option<Cost>, tokens: Option<Tokens> }` for the summary.
- Stop lines: `stopped: spend limit $<limit> reached ($<spent> spent); …` (unchanged) and
  `stopped: token limit <limit> reached (<used> used); input from line L on was not processed`.

## Run start check (new)

Before input is read: provider `TypeSafe` and `max_cost` `At(_)` → `SpendLimitUnsupported`, exit 2,
one of:

- the top level sets `max-cost = V`: `max-cost = V in the config file applies to every provider, but
  TypeSafe reports no cost; keep it for OpenRouter with jevpipe config set openrouter.max-cost V and
  jevpipe config unset max-cost, and limit TypeSafe runs with max-tokens`;
- otherwise (the flag): `TypeSafe reports no cost, so --max-cost cannot be enforced; use
  --max-tokens instead`.

## Summary (changed, `src/summary.rs`)

`finish(reported, stop)`: shows `<abbreviated> tokens` when `reported.tokens` is set and `$<cost>`
when `reported.cost` is set, each independently.

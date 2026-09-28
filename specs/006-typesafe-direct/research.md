# Research: TypeSafe's Own API as a Second Provider

All findings checked on 2026-09-28: TypeSafe's docs (docs.typesafe.ai: `api.md`, `models.md`,
`sdk/python/api/constants.md`, `sdk/python/api/retries.md`, `sdk/python/api/exceptions.md`,
`sdk/javascript/api/interfaces/Usage.md`), OpenRouter's live OpenAPI spec
(`openrouter.ai/openapi.yaml`, operation `createSystemone`), and live requests to both services
(`scratch/typesafe_probe.py`, 14 requests, well under a cent).

## R1. TypeSafe's wire format, confirmed live

- **Decision**: speak to TypeSafe with the request jevpipe already sends; read `answers` and `usage`
  from the reply exactly as for OpenRouter, with `usage.cost` absent.
- **Evidence** (same state and questions on both services):

  | | OpenRouter | TypeSafe |
  |---|---|---|
  | endpoint | `POST https://openrouter.ai/api/v1/systemone` | `POST https://api.typesafe.ai/v1/systemone` |
  | auth | `Authorization: Bearer <OpenRouter key>` | `Authorization: Bearer <TypeSafe key>` |
  | reply `model` | `typesafe/jev-1.13-20260917` | `jev-1.13.0` |
  | answers | identical (noul 0.97, choice `fix`, confidence 1) | identical |
  | `usage` | `{"input_tokens":355,"output_tokens":55,"cost":0.00001491}` | `{"input_tokens":355,"output_tokens":55}` |
  | extra fields | `id`, `provider` | none; header `x-typesafe-request-id` |
  | state as `{path, content}` | works (002) | works |

- **Rationale**: the answers are the same model's; only the envelope differs in `cost`, which
  jevpipe already treats as optional, so no request or answer code changes.
- **Price check**: 355 input tokens × $0.042 per million = $0.00001491, exactly OpenRouter's reported
  cost; output tokens are free on both (TypeSafe `models.md`: "$0.042 per million tokens … Output
  tokens are free").

## R2. Model names

- **Decision**: one built-in default model, `jev-latest`, for both providers (replacing
  `~typesafe/jev-latest`). The `--model` help names the pinned form of each provider.
- **Evidence**:

  | model | OpenRouter | TypeSafe |
  |---|---|---|
  | `jev-latest` | 200 (→ `typesafe/jev-1.13-20260917`) | 200 (→ `jev-1.13.0`) |
  | `~typesafe/jev-latest` | 200 | 400 `Unknown model: ~typesafe/jev-latest` |
  | `jev-1.13` | 200 (one transient 520 first) | 400 `Unknown model: jev-1.13` |
  | `typesafe/jev-1.13` | 200 | not tried (prefixed names are unknown) |
  | `jev-1.13.0` | 400 `Model typesafe/jev-1.13.0 does not exist` | 200 |

  OpenRouter documents the mapping ("Bare System One model IDs such as `jev-1.13` and `jev-latest`
  are mapped onto the `typesafe/` namespace"; `jev-latest` is routed as `~typesafe/jev-latest`).
- **Rationale**: a shared default means the model's default does not depend on the provider, so
  `settings` stays a plain clap struct with a static default. The same model answers either way.
- **Alternatives considered**: a default per provider (`~typesafe/jev-latest` / `jev-latest`) keeps
  0.1.1's spelling but makes the model default depend on the config's provider (dynamic clap
  default, provider-aware `config list`) for no behavioural gain.

## R3. Error replies

- **Decision**: read the message from either error shape; classify by status as today.
- **Evidence** (TypeSafe; FastAPI style):

  | case | status | body |
  |---|---|---|
  | unknown model | 400 | `{"detail":{"error_type":"api_usage_error","message":"Unknown model: jev-1.13"}}` |
  | bad key | 401 | `{"detail":{"error_type":"authentication_error","message":"Cannot authenticate with the server. Please check your API key and try again."}}` |
  | missing field | 422 | `{"detail":[{"type":"missing","loc":["body","questions"],"msg":"Field required",…}]}` |
  | record too large (~40k tokens) | 400 | `{"detail":{"error_type":"max_tokens_exceeded"}}` |

  OpenRouter: `{"error":{"message":"…","code":400}}` (unchanged).
- **Rationale**: today a TypeSafe refusal would print only `400 Bad Request`; the user needs
  `Unknown model: jev-1.13`. The too-large case already works: the existing rule
  `400 if body.contains("max_tokens_exceeded")` matches TypeSafe's body, so the record fails alone.
  The message is `detail.message`, else `detail.error_type`; any other body (such as the 422
  validation list, which jevpipe never provokes) falls back to the status alone, as today.
- **Not probed**: 429 (needs >1,200 requests per minute) and an empty balance (needs a drained
  account). TypeSafe documents 429 and 529 as retryable and its SDK honours `Retry-After` and
  `retry-after-ms`; jevpipe retries 429/529 already and honours `Retry-After`. `retry-after-ms` is
  not added: the exponential backoff covers it (lean). No credit error is documented, so an empty
  balance is a run-level error with TypeSafe's message (spec edge case).

## R4. Keys

- **Decision**: per provider, the provider's own variable (`OPENROUTER_API_KEY`,
  `TYPESAFE_API_KEY`) and its own keychain entry: service `jevpipe`, user `openrouter-api-key`
  (unchanged, so stored 0.1.x keys keep working) and `typesafe-api-key`.
- **Rationale**: `TYPESAFE_API_KEY` is what TypeSafe's SDKs read (`constants.md`), so a user who set
  it for the SDK is already done. Separate entries make switching providers free of re-entering keys.
- **Alternatives considered**: one neutral `JEVPIPE_API_KEY` for whichever provider is active —
  rejected: the same key would silently be sent to the wrong service after a provider switch, and
  users' existing variables would not work.
- **Missing-key message**: fixed text per provider that names its variable, `auth set-key`, and
  the command switching to the other provider, e.g. `no TypeSafe API key: set TYPESAFE_API_KEY or
  run jevpipe auth set-key (provider typesafe; jevpipe config set provider openrouter switches)`.
- **Alternatives considered**: a hint only when the other provider's variable is set — rejected in
  review: it reads the environment on a failing path, needs its own tests, and a developer's own
  permanent variables could change test output; the fixed text says the same every time.

## R5. Token counts: parsing and display

- **Decision**: parse `--max-tokens` with `parse-size` 1.1 (MIT, no dependencies, ~20M downloads);
  display token totals in the summary with `unit-prefix` 0.5 (MIT, no dependencies, the crate
  `indicatif` uses). Both prototyped in `scratch/tokfmt`.
- **Evidence**: `parse-size` with its default (decimal) config: `5M`, `5m` → 5,000,000; `250k`,
  `250K` → 250,000; `1.5M` → 1,500,000; `1000000`, `1_000`, `1e6` parse; `-1`, `abc` fail; `0` parses
  (rejected by jevpipe: a limit must be positive); byte suffixes are tolerated (`5MB` → 5,000,000,
  `5Mi` → 5,242,880) and `2.5` rounds to 3. `unit-prefix`: 812 → `812`, 41,300 → `41.3k`,
  12,500,000 → `12.5M`; 999,999 → `1000.0k` (cosmetic, accepted).
- **Rationale**: the project prefers a proven crate over hand-rolling; the tolerated byte suffixes
  are harmless for a count, and the help names the plain forms only.
- **Alternatives considered**: `human_format` (same output, same 999,999 wart, one more feature
  surface); plain integers only (no dependency, but `--max-tokens 5000000` is error-prone to type);
  exact totals in the summary (`12500000 tokens` is hard to read at a glance). The stop line shows
  exact numbers, where precision matters.

## R6. Counting tokens and enforcing the limits

- **Decision**: `Limits` holds two instances of one generic `Meter` (a `u64` total, whether any
  answer reported it, and its `Limit`): spend in nano-dollars and tokens. Each answer adds its cost
  and `input_tokens + output_tokens` to them. `may_send` asks both meters; the first reached sets the
  stop, `Stop::Limit(Measure)` naming which. An answer missing a measure whose limit is set is one
  run-level error, `Unreported(Measure)`, replacing `NoCost`.
- **Rationale**: cost and tokens behave identically (soft stop, overshoot bounded by the requests in
  flight, the same stop path: prefix output, stop line, resume line, exit 3), so they share one
  implementation instead of two parallel sets of fields, errors and stop variants.
- **Spend limit on TypeSafe**: rejected when the run starts (before input is read): the provider is
  known up front not to report cost, so failing at the first answer (today's `NoCost`) would spend
  up to `--concurrency` requests first. `NoCost` stays for gateways reached through `base-url` that
  omit cost.

## R7. Summary

- **Decision**: after the counts, the summary shows each measure some answer reported: tokens
  (`41.3k tokens`), then dollars (`$0.0017`). TypeSafe runs show tokens, OpenRouter runs both.
- **Rationale**: one rule for every provider and gateway, no either/or; tokens are what
  `--max-tokens` limits, so showing them on OpenRouter too helps choose a limit. A placeholder such
  as `---$` carries no information and could be misread by an agent parsing the line.
- **Alternatives considered**: dollars, else tokens (keeps OpenRouter's line as in 0.1.1, but is a
  special case with less information).

## R8. Where the provider lives

- **Decision**: `provider` is a config key like `base-url` (not a flag), validated against
  `openrouter | typesafe`, default `openrouter`. It decides the default `base-url`, the key variable,
  the keychain entry, the prompt and messages naming the provider, and whether a spend limit is
  allowed. `base-url` still overrides the address (gateways, test stand-in).
- **Rationale**: the provider goes together with a key and is chosen once per machine; per-run
  switching is out of scope (spec). Tests set it in the temp config file, as they set `base-url`.
- **`auth` needs the config now**: `auth set-key`/`remove-key` act on the configured provider's
  entry, so they load the config; with a broken config file they fail naming the file (exit 2),
  which `config path|set|unset` can repair.

## R9. Settings per provider

- **Decision**: the config file may hold a TOML table per provider (`[openrouter]`, `[typesafe]`)
  with any config key except `provider`. Loading reads the top-level keys (scope: every provider)
  and both tables (scope: that provider), validates all of them, and exposes the merged settings of
  the active provider: its table over the top level. Everything downstream (clap defaults, `--help`,
  the pipeline) keeps receiving one flat list, unchanged.
- **Keys on the command line**: `<provider>.<key>` (`openrouter.max-cost`) addresses a table; this
  is TOML's own dotted-key syntax, so `config set` writes exactly what a user would type by hand.
  `toml_edit` creates the table on `set`; `unset` removes a table left empty. `config get
  <provider>.<key>` resolves as if that provider were active.
- **Rejected in a table**: `provider` (it chooses the table), `max-cost` under `typesafe` (TypeSafe
  reports no cost; the error names `typesafe.max-tokens`), and any table other than the two
  providers (unknown key, listing the valid keys and the two prefixes).
- **Spend limit on TypeSafe**: refused before any request when the effective `max-cost` is set (it
  can only come from the command line or the top level, since the `typesafe` table cannot hold it).
  The message is chosen from the config, without tracking where clap took the value from: when the
  top level sets `max-cost`, it says so and gives the two commands that move it to
  `openrouter.max-cost`; otherwise it names `--max-tokens`. `--max-cost none` lifts either for one
  run.
- **Rationale**: settings that only one provider understands (a spend limit, a pinned model name:
  `jev-1.13` vs `jev-1.13.0`) otherwise block or break runs after a provider switch, forcing the
  user to unset and re-set them. With tables, a switch is one command, and the strict refusal always
  has a precise fix instead of "unset your guard".
- **Alternatives considered**: ignoring a configured spend limit on TypeSafe (silently drops a guard
  the user expects to hold); only the strict refusal without tables (safe, but switching back and
  forth means unsetting and re-setting the limit each time); one config file per provider (more
  files and paths to explain, and shared settings would be duplicated).

## R10. Test isolation from the developer's own keys

- **Finding**: the maintainer's machine now has `TYPESAFE_API_KEY` as a user environment variable
  (set for this research). Binary tests inherit the environment, so key
  lookup could see it.
- **Decision**: every binary test helper removes both key variables and then sets only the one the
  test means (`env_remove` before `env`), as it already sets `JEVPIPE_CONFIG`.

## R11. Release and docs

- **Decision**: version 0.2.0 (new features, one visible default spelling change). README: install
  section names both keys (with links to both key pages), a short paragraph on provider sections for
  people using both, "Cost and speed" describes billing on
  either provider and `--max-tokens`, the license note says "through OpenRouter or TypeSafe's own
  API". Agent skill: `compatibility` names both services, a line on `--max-tokens`. `CLAUDE.md`:
  the API key rule names both variables. Links stay absolute (README is the PyPI description).

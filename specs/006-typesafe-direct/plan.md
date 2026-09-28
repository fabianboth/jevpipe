# Implementation Plan: TypeSafe's Own API as a Second Provider

**Branch**: `006-typesafe-direct` | **Date**: 2026-09-28 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/006-typesafe-direct/spec.md`

## Summary

TypeSafe becomes a second provider next to OpenRouter. The live probes (research R1) showed the same
request and answers on both; only the key, the address, the usage report (tokens, no cost) and the
error shape differ. So the change is mostly configuration and accounting, not a second client:

1. **Provider.** A `provider` config key (`openrouter` default, `typesafe`) picks the default
   address, the key variable (`OPENROUTER_API_KEY` / `TYPESAFE_API_KEY`) and the keychain entry.
   The default model becomes `jev-latest`, which both accept.
2. **Keys.** The key lookup, `auth set-key|remove-key` and `config list` take the provider. A
   missing key gets a fixed message naming the provider, its key and how to switch.
3. **Tokens.** `Limits` becomes two instances of one generic `Meter` (spend, tokens). Each reply's
   `input_tokens + output_tokens` feeds the token meter, and `--max-tokens` (parsed by `parse-size`)
   is its limit on the existing stop path (exit 3). The summary shows each reported measure: tokens
   (`unit-prefix`), and dollars where reported. A spend limit on TypeSafe is refused before the run.
4. **Settings per provider.** The config file may hold `[openrouter]` and `[typesafe]` tables whose
   keys beat the top level for that provider (`config set typesafe.model jev-1.13.0`). Loading merges
   the active provider's view into the same flat list the rest of the program reads today. A spend
   limit that would apply on TypeSafe is refused before the run, with the commands that move it to
   `openrouter.max-cost`.
5. **Errors.** Error messages are read from TypeSafe's `{"detail": {"message"}}` as well as
   OpenRouter's `{"error": {"message"}}`.
6. **Docs.** README, help, skill and `CLAUDE.md` name both providers. Release 0.2.0.

## Technical Context

**Language/Version**: Rust 1.98 (edition 2024), pinned in `rust-toolchain.toml`
**Primary Dependencies**: existing; new: `parse-size` 1.1 (MIT, no dependencies) for counts,
`unit-prefix` 0.5 (MIT, no dependencies) for the summary's abbreviated tokens
**Storage**: one more keychain entry (`jevpipe` / `typesafe-api-key`); config gains `provider`,
`max-tokens` and a table per provider
**Testing**: `cargo test`; binary tests against the `wiremock` stand-in, which gains a TypeSafe mode
(tokens, no cost, `{"detail": …}` errors); keychain lookup per provider in-process against the mock
store; the quickstart with real keys
**Target Platform**: Linux, Windows, macOS (CI matrix); single binary
**Project Type**: single CLI crate (library + thin binary)
**Performance Goals**: unchanged (one more atomic add per answer)
**Constraints**:
- a key is never sent to the other provider;
- no key in any output;
- tests are independent of the developer's `OPENROUTER_API_KEY` and `TYPESAFE_API_KEY`;
- OpenRouter behaviour is unchanged apart from the default model's spelling.
**Scale/Scope**: 2 new modules (`provider`, `tokens`), ~12 changed, ~450 lines incl. tests

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass. Out: a second client, dollar estimates from price tables, balance queries, a per-run provider flag, auto-detection, `retry-after-ms`. Provider sections are in because without them a guard or pinned model set for one provider blocks the other; they stay inside `config/`. |
| II. Automated Verification | Pass. Every story is covered offline: the stand-in plays both providers; the keychain per provider runs against the mock store; tests clear both key variables. `check.ps1` gates format, clippy, tests and `cargo deny`. The real services are covered by the quickstart and were probed during research. |
| III. Reusable Components | Pass. One `Provider` enum feeds config, auth, prompt and messages. The token limit reuses `Limit<T>`, `Limits` and the stop path of `--max-cost`. The error parsing extends the existing `ErrorBody`. The crates replace hand-written number parsing and formatting. |

Post-design re-check: unchanged. Both new crates are dependency-free and MIT, which `deny.toml`
allows.

## Project Structure

### Documentation (this feature)

```text
specs/006-typesafe-direct/
├── spec.md
├── plan.md
├── research.md          # R1–R11: live probes, model names, errors, keys, token crates, limits, summary, provider, provider sections, test isolation, docs
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md           # --max-tokens, --model default, provider key, TYPESAFE_API_KEY, auth per provider, summary, new errors
│   └── service.md       # both endpoints and keys, usage tokens, both error shapes
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── provider.rs          # NEW: Provider enum (openrouter | typesafe): name, default base URL, key variable, keychain user, reports cost, other(); FromStr for the config value
├── tokens.rs            # NEW: Tokens (u64): parse via parse-size (positive), exact Display, abbreviated() via unit-prefix
├── lib.rs               # CHANGED: modules; auth::run gets the config (provider)
├── cli.rs               # CHANGED: help texts (config: provider; auth: both variables; exit 3 incl. token limit)
├── settings.rs          # CHANGED: model default jev-latest with the new help; --max-tokens (Limit<Tokens>)
├── config/
│   ├── mod.rs           # CHANGED: entries with a Scope; provider(); view(provider) merges table over top level; setting/settings/base_url read the active view; Origin::ConfigFile(Scope); shared_spend_limit()
│   ├── keys.rs          # CHANGED: ScopedKey (<provider>.<key>); config-only keys base-url and provider (top level only); typesafe.max-cost rejected; defaults via the provider
│   ├── file.rs          # CHANGED: read top level + provider tables; set/unset/get on a ScopedKey (create table, drop empty table)
│   └── command.rs       # CHANGED: get <provider>.<key>; list origins with scope; API key line for the configured provider
├── auth/
│   ├── mod.rs           # CHANGED: api_key(provider), source(provider), messages naming the provider and how to switch
│   ├── keychain.rs      # CHANGED: entry(provider) with the provider's keychain user
│   ├── prompt.rs        # CHANGED: prompt names the provider
│   ├── command.rs       # CHANGED: set-key / remove-key on the configured provider; confirmation names it
│   └── tests.rs         # CHANGED: lookup per provider; both entries independent
├── limits.rs            # CHANGED: generic Meter (total, reported, limit) used for spend and tokens; Measure; Stop::Limit(Measure); Unreported(Measure) replaces NoCost; add(Usage); reported()
├── pipeline.rs          # CHANGED: ServiceConfig from provider + key; spend-limit check before the run; limits.add(reply.usage)
├── service/
│   ├── mod.rs           # CHANGED: Usage { tokens, cost } from input_tokens/output_tokens/cost; Reply carries Usage
│   └── error.rs         # CHANGED: message from {"error": {…}} or {"detail": {…} | […]}
├── summary.rs           # CHANGED: abbreviated tokens and dollars, each when reported
└── (all other modules unchanged)

tests/
├── cli/
│   ├── home.rs          # CHANGED: clear both key variables, then set the one meant
│   ├── help.rs          # CHANGED: new options, provider, both variables
│   ├── config.rs        # CHANGED: provider key (valid, invalid, base-url default follows it), max-tokens values, provider sections (set/get/unset/list, precedence, rejected keys, comments kept)
│   ├── usage.rs         # CHANGED: --max-tokens values; --max-cost refused with provider typesafe
│   └── auth.rs          # CHANGED: missing key per provider
└── pipeline/
    ├── stand_in.rs      # CHANGED: clear both variables; TypeSafe mode (config provider + base-url, TYPESAFE_API_KEY, usage without cost, detail errors); records the bearer key; markers tokens=<n>, notokens
    ├── providers.rs     # NEW: user story 1 (key and model sent, outputs equal across providers, service messages) and 2 (summary dollars vs tokens)
    └── limits.rs        # CHANGED: user story 3 (token limit, both limits, no tokens, in flight); user story 4 (section limits follow the provider; top-level max-cost refused on TypeSafe with the move commands)
```

**Structure Decision**: two new flat modules, one per new concept (`provider`, `tokens`), next to
`cost` and `limits` that they mirror. No new client: `Service` is provider-agnostic apart from the
URL and key it is built with.

### Other repository changes

- `Cargo.toml`: `parse-size = "1.1"`, `unit-prefix = "0.5"`; version `0.2.0` at release.
- `README.md`:
  - Install: "your OpenRouter or TypeSafe key", both key pages linked, `config set provider typesafe`.
  - Cost and speed: billing on either provider, `--max-tokens`, `--max-cost` needs OpenRouter.
  - License note: "through OpenRouter or TypeSafe's API".
- `skills/jevpipe/SKILL.md`: `compatibility` names both services; `--max-tokens` next to `--max-cost`.
- `CLAUDE.md`: the API key rule names both variables (`OPENROUTER_API_KEY` or `TYPESAFE_API_KEY`, by
  provider).

## Code Structure

1. **Provider as data, not a code path.** `Provider` answers questions: address, variable, keychain
   user, name, whether it reports cost. Nothing branches on it except those answers and the one
   run-start check. `Service` stays one client.
2. **Config-only keys.** `keys.rs` treats `base-url` and `provider` alike: validated by their own
   parser, not flags. Their defaults depend on the loaded provider, so `Config::setting` asks the
   provider for the `base-url` default.
3. **Usage is one value, measures are one type.** The reply carries `Usage { tokens, cost }`,
   `Limits::add` feeds it into two `Meter`s, and the summary reads `Reported { cost, tokens }` back.
   A meter knows nothing about dollars or tokens; `Measure` supplies the flag name and formatting,
   so limit checks, "cannot be enforced" errors and stop lines exist once.
4. **Refuse early what cannot work.** A spend limit with a provider that reports no cost fails
   before input is read, so no request is spent. `NoCost` remains for gateways reached through
   `base-url`.
5. **Scopes stop at loading.** Only `config/` knows about provider tables: it validates every entry
   with its scope and hands out the active provider's merged view. clap defaults, `--help`,
   `config get <key>` and the pipeline see the same flat settings as before, so the feature adds no
   branches outside `config/` except the refusal message, which asks `Config` whether the spend
   limit is a shared one.
6. **One error message extractor.** `ErrorBody` becomes an untagged enum of the two shapes
   (`error.message`, `detail.message` / `detail.error_type`) with one `message()`; any other body
   falls back to the status. `limit_source` stays OpenRouter's. Classification by status is
   unchanged.
7. **Tests own their environment.** Every binary helper removes `OPENROUTER_API_KEY` and
   `TYPESAFE_API_KEY` before setting the one it means (research R10).

## Complexity Tracking

No violations.

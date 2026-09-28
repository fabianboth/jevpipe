---
description: "Task list for TypeSafe's own API as a second provider"
---

# Tasks: TypeSafe's Own API as a Second Provider

**Input**: Design documents from `specs/006-typesafe-direct/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/cli.md](contracts/cli.md),
[contracts/service.md](contracts/service.md), [quickstart.md](quickstart.md)

**Tests**: Required by FR-017. Integration tests run the real binary (`assert_cmd`) against the
`wiremock` stand-in, with `JEVPIPE_CONFIG` pointing at a temp file whose `base-url` points at the
stand-in; no network, no real key, no real keychain. The stand-in plays OpenRouter (tokens and cost,
`{"error": …}` bodies) or TypeSafe (tokens only, `{"detail": …}` bodies). Every binary test clears
`OPENROUTER_API_KEY` and `TYPESAFE_API_KEY` first (R10): the maintainer's machine has both set. The
keychain per provider is tested in-process against `keyring_core::mock::Store` in
`src/auth/tests.rs`. Write each story's tests first and see them fail.

**Rules for every task**: follow `CLAUDE.md`:
- no comments;
- max 3 parameters besides `self`;
- exhaustive `match` without `_` on enums, and `matches!` for `#[non_exhaustive]` foreign enums;
- no `unwrap`/`expect`/`panic!` in `src/`, and no `unsafe`;
- private by default, one module per concept;
- clap help text goes in doc comments on the clap types.

`./check.ps1 -Fix` passes at the end of each phase. "Point N" refers to plan.md, "Code Structure";
"R N" to research.md. Wire-format facts come from research.md R1–R3 (checked live), never from
guesses.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1 TypeSafe key, US2 summary, US3 token limit, US4
  settings per provider)

---

## Phase 1: Setup

**Purpose**: dependencies.

- [X] T001 Add `parse-size@1.1` and `unit-prefix@0.5` with `cargo add` in `Cargo.toml` (R5); run `cargo deny check` and confirm both pass `deny.toml` (MIT, no new transitive crates)

**Checkpoint**: `./check.ps1 -Fix` passes with the unchanged product.

---

## Phase 2: Foundational

**Purpose**: the two new value types, the provider as a config key, usage with tokens, the meter
refactor of `Limits`, and a stand-in that can play either provider. **No story work before this
phase is done.** Existing behaviour stays the same except the default model's spelling.

- [X] T002 [P] Create `src/provider.rs` (data-model.md Provider): `enum Provider { OpenRouter, TypeSafe }` with `FromStr`/`Display` for the config values `openrouter`/`typesafe` (error: `the providers are openrouter, typesafe`), `name()` (`OpenRouter`/`TypeSafe`), `default_base_url()` (`https://openrouter.ai/api`, `https://api.typesafe.ai`), `key_variable()` (`OPENROUTER_API_KEY`, `TYPESAFE_API_KEY`), `keychain_user()` (`openrouter-api-key`, `typesafe-api-key`), `reports_cost()` (true, false), `other()`; `Default` is `OpenRouter`; register the module in `src/lib.rs`
- [X] T003 [P] Create `src/tokens.rs` (data-model.md Tokens, R5): `Tokens(u64)` with `FromStr` via `parse_size::Config::new().parse_size` (errors reworded to `not a count, for example 250k or 5M`), `Display` exact (`3300`), `abbreviated()` via `unit_prefix::NumberPrefix::decimal` (`812`, `41.3k`, `12.5M`: one decimal with a prefix, none without), `count()` and `from_count()`; register the module in `src/lib.rs`
- [X] T004 Add the `provider` config key (R8, data-model.md Config): in `src/config/keys.rs` a second config-only key next to `base-url`, validated by `Provider::from_str`, default `openrouter`; the default of `base-url` comes from the provider in force; in `src/config/mod.rs` `Config::provider()` (the file's value or the default) and `Config::base_url()` falling back to `provider.default_base_url()`; `config list`/`config get base-url` show that effective default
- [X] T005 Change the model default to `jev-latest` in `src/settings.rs` (R2) with the doc comment `Model to ask; pin a version such as jev-1.13 (OpenRouter) or jev-1.13.0 (TypeSafe)`; update every test that expects `~typesafe/jev-latest` (`tests/cli/config.rs`, `tests/cli/help.rs`, `tests/pipeline/*.rs`)
- [X] T006 Read tokens from each reply (data-model.md Usage, contracts/service.md Usage): in `src/service/mod.rs` `Usage { tokens: Option<Tokens>, cost: Option<Cost> }` from `usage.input_tokens + usage.output_tokens` (both present, else `None`; a non-integer or negative count is an unexpected-answer error) and `usage.cost`; `Reply { answers, usage }` replaces `Reply { answers, cost }`
- [X] T007 Refactor `src/limits.rs` into meters (point 3, R6, data-model.md Meter/Limits): a private generic `Meter` (`total: AtomicU64`, `reported: AtomicBool`, `limit: Limit<u64>`) with `add(Option<u64>) -> Result<(), Unreported>`, `reached()`, `reported()`; `enum Measure { Spend, Tokens }` naming the flag (`--max-cost`, `--max-tokens`) and formatting amounts (`$0.5` via `Cost`, `5000000` via `Tokens`); `Limits` holds `spend` and `tokens` meters (the token limit is `Limit::Unlimited` until US3); `add(usage: &Usage)` feeds both; `Unreported(Measure)` replaces `NoCost` with the same text for spend (`the service reported no cost, so --max-cost cannot be enforced`); `Stop::Limit(Measure)` replaces `Stop::Spend` with an unchanged spend stop line; `reported()` returns `Reported { cost: Option<Cost>, tokens: Option<Tokens> }`; update `src/pipeline.rs` (`limits.add(&reply.usage)`, `NoCost` → `Unreported`) and `src/summary.rs` (`finish(reported, stop)`, still printing only the cost until US2)
- [X] T008 Make the tests own their keys and give the stand-in a TypeSafe mode (R10, contracts/service.md): in `tests/cli/home.rs` and `tests/pipeline/stand_in.rs` call `env_remove` for both key variables before setting the one meant; `StandIn` gains a provider mode set at start (`StandIn::start()` = OpenRouter as today, `StandIn::typesafe()` writes `provider = "typesafe"` next to `base-url`, sets `TYPESAFE_API_KEY=test-typesafe-key`, answers `usage` without `cost`, and error bodies as `{"detail": {"error_type": "api_usage_error", "message": "…"}}`); record each request's `Authorization` header for assertions; new markers `tokens=<n>` (set `input_tokens` to n and `output_tokens` to 0) and `notokens` (usage without token counts)

**Checkpoint**: all existing tests pass (the model default updated); `./check.ps1 -Fix` passes.

---

## Phase 3: User Story 1 - Run jevpipe on a TypeSafe key (Priority: P1) 🎯 MVP

**Goal**: with `provider = "typesafe"` and a TypeSafe key (variable or keychain), `filter` and `map`
run against TypeSafe; keys never cross providers; the service's own messages reach the user.

**Independent Test**: `StandIn::typesafe()` with `TYPESAFE_API_KEY`: `filter` and `map` over a few
lines produce the same output as against the OpenRouter stand-in, and the stand-in saw
`Bearer test-typesafe-key` and `"model": "jev-latest"`.

### Tests for User Story 1

- [X] T009 [P] [US1] Create `tests/pipeline/providers.rs` (register in `tests/pipeline/main.rs`), covering spec US1 scenarios 1, 3, 6 and 8:
  - `filter` and `map` against `StandIn::typesafe()` send `Bearer test-typesafe-key` and `jev-latest`, and print the same output as against `StandIn::start()` (scenario 1);
  - with the provider `typesafe` and only `OPENROUTER_API_KEY` set, the run fails before any request with the TypeSafe missing-key message, the stand-in receives nothing, and the exit status is 2 (scenario 3);
  - `--model jev-1.13.0` is sent as given (scenario 6);
  - a `{"detail": {"message": "Unknown model: jev-1.13"}}` 400 ends the run with `service error: 400 Bad Request: Unknown model: jev-1.13`, exit 2; a detail body with only `error_type` shows the type; a `{"detail": [...]}` body shows the status alone (scenario 8, contracts/service.md "Error body")
- [X] T010 [P] [US1] Extend `tests/cli/config.rs` and `tests/cli/auth.rs`:
  - `config set provider typesafe` is accepted, `config set provider anthropic` is rejected with `the providers are openrouter, typesafe` and the file is unchanged;
  - with the provider `typesafe`, `config list` shows `provider = "typesafe"  # config file`, `base-url = "https://api.typesafe.ai"  # default` and `# API key: from TYPESAFE_API_KEY`;
  - `config get base-url` follows the provider;
  - a run without any key fails with the fixed message of the provider in force, for each provider (contracts/cli.md, FR-008);
  - with the provider `openrouter` and only `TYPESAFE_API_KEY` set, the message is still the OpenRouter one, naming `jevpipe config set provider typesafe` (scenario 5)
- [X] T011 [P] [US1] Extend `src/auth/tests.rs` (in-process, mock store): a key stored for TypeSafe is found for TypeSafe and not for OpenRouter and vice versa; `TYPESAFE_API_KEY` wins over the stored TypeSafe key and is ignored for OpenRouter; removing one provider's key leaves the other's (scenario 2, FR-006)

### Implementation for User Story 1

- [X] T012 [US1] Key lookup per provider in `src/auth/mod.rs` and `src/auth/keychain.rs` (data-model.md ApiKey lookup, R4): `api_key(provider)`, `source(provider)` and `keychain::entry(provider)` with `provider.keychain_user()` (OpenRouter's entry unchanged, so 0.1.x keys keep working); `LookupError::Missing(Provider)` with the text `no <Name> API key: set <VARIABLE> or run jevpipe auth set-key (provider <p>; jevpipe config set provider <other> switches)`; `Source::Environment` shows the provider's variable; keychain-unavailable messages name the provider's variable
- [X] T013 [US1] `auth` commands per provider (FR-007, contracts/cli.md `auth`): `src/lib.rs` passes the config to `auth::run`, which fails with the config error (exit 2) when the file is broken; `src/auth/command.rs` stores/removes the configured provider's key with confirmations `jevpipe: <Name> API key stored in the keychain` / `jevpipe: no <Name> API key stored`; `src/auth/prompt.rs` prompts `<Name> API key: ` (and the visible variant)
- [X] T014 [US1] Use the provider in the run: `src/pipeline.rs` builds `ServiceConfig` from `config.base_url()` and `auth::api_key(config.provider())`; `src/config/command.rs` prints `# API key: <source>` for `config.provider()`
- [X] T015 [P] [US1] Read both error shapes in `src/service/error.rs` (point 6, R3, contracts/service.md): `ErrorBody` becomes an untagged enum of `{"error": {message, metadata}}` and `{"detail": {message?, error_type?}}` with one `message()` (`detail.message`, else `detail.error_type`); any other body falls back to the status; `limit_source` stays on the OpenRouter shape; classification by status unchanged
- [X] T016 [US1] Help texts in `src/cli.rs`: `config --help` names the `provider` key and its values and that `base-url` defaults to the provider's address; `auth --help` and `set-key` examples name both variables and say the commands act on the configured provider's key; update `tests/cli/help.rs` expectations

**Checkpoint**: US1 tests pass; a TypeSafe key works end to end against the stand-in.

---

## Phase 4: User Story 2 - See what a run used on either provider (Priority: P2)

**Goal**: the summary shows the reported tokens and, where reported, the dollars.

**Independent Test**: `map` over 10 lines against `StandIn::typesafe()` ends with `…, <n> tokens,
…s`; against `StandIn::start()` with `…, <n> tokens, $…, …s`.

### Tests for User Story 2

- [X] T017 [P] [US2] Extend `tests/pipeline/stderr.rs`: the TypeSafe summary shows tokens and no dollars (`3.3k tokens` for 10 lines with `tokens=330`); the OpenRouter summary shows tokens then dollars; with `notokens nocost` neither appears; 812 tokens show as `812 tokens`; update existing summary expectations that now gain a token total (spec US2 scenarios 1–3, FR-011)

### Implementation for User Story 2

- [X] T018 [US2] In `src/summary.rs` print `, <tokens.abbreviated()> tokens` when `reported.tokens` is set and then `, <cost>` when `reported.cost` is set, each independently (R7, contracts/cli.md "Standard error")

**Checkpoint**: US1 and US2 tests pass.

---

## Phase 5: User Story 3 - Limit a run by tokens (Priority: P3)

**Goal**: `--max-tokens` / `max-tokens` stops a run like `--max-cost`; a spend limit on TypeSafe is
refused before any request.

**Independent Test**: against a stand-in reporting 330 tokens per answer with `--concurrency 1`,
`map --max-tokens 3300` over 100 lines prints the first 10 answers, names the token limit and line 11,
and exits 3.

### Tests for User Story 3

- [X] T019 [P] [US3] Extend `tests/pipeline/limits.rs` (spec US3 scenarios 1–5):
  - `--max-tokens 3300` with `tokens=330` and `--concurrency 1` against both stand-ins stops after 10 records with `stopped: token limit 3300 reached (3300 used); input from line 11 on was not processed`, exit 3, and a rerun from line 11 completes the output;
  - in-flight answers count and print as for `--max-cost`;
  - with `--max-cost` and `--max-tokens` both set, whichever is reached first names itself;
  - `--max-tokens` with `notokens` answers ends with `the service reported no token count, so --max-tokens cannot be enforced`, exit 2;
  - `StandIn::typesafe()` with `--max-cost 0.1`, and separately with `max-cost` in the config's top level, fails with `TypeSafe reports no cost, so --max-cost cannot be enforced; use --max-tokens instead`, exit 2, and the stand-in received no request;
  - `--max-cost none` lifts the configured one
- [X] T020 [P] [US3] Extend `tests/cli/usage.rs` and `tests/cli/config.rs` (scenario 6):
  - `--max-tokens` accepts `250k`, `5M`, `1.5M`, `1000000` and `none`;
  - it rejects `0` (`must be positive`), `-1` and `lots` (`not a count, for example 250k or 5M`) before any input is read;
  - `config set max-tokens 5M` stores it and `config get max-tokens` prints `5M`

### Implementation for User Story 3

- [X] T021 [US3] Add `max_tokens: Limit<Tokens>` to `src/settings.rs` (`--max-tokens <COUNT|none>`, default `none`, doc comment `Stop sending requests once this run's reported tokens reach COUNT, e.g. 250k or 5M`, parser via `limit(count)` rejecting zero with `must be positive`); `Limits::new` gives the token meter this limit (`src/limits.rs`), so the stop line, `Unreported(Measure::Tokens)` and the resume logic come from T007's meter
- [X] T022 [US3] Refuse a spend limit before the run in `src/pipeline.rs` (R6, data-model.md Run start check): when `!config.provider().reports_cost()` and `settings.max_cost` is `Limit::At(_)`, fail with `SpendLimitUnsupported` (`TypeSafe reports no cost, so --max-cost cannot be enforced; use --max-tokens instead`), exit 2, before reading input or sending a request
- [X] T023 [US3] Help in `src/cli.rs`: the exit-status text for 3 lists the token limit next to the spend and time limits (`filter`, `map`); update `tests/cli/help.rs`

**Checkpoint**: US1–US3 tests pass.

---

## Phase 6: User Story 4 - Keep settings per provider (Priority: P4)

**Goal**: `[openrouter]` and `[typesafe]` tables in the config file override the top level for their
provider; `config set|get|unset` take `<provider>.<key>`; the spend-limit refusal shows how to move a
shared limit.

**Independent Test**: with `openrouter.max-cost 0.001` and `typesafe.max-tokens 3300` configured, a
`map` run against the OpenRouter stand-in stops at the spend limit; after `config set provider
typesafe`, a run against the TypeSafe stand-in stops at the token limit; no other config change.

### Tests for User Story 4

- [X] T024 [P] [US4] Extend `tests/cli/config.rs` (spec US4 scenarios 2, 4–8, contracts/cli.md "Provider sections"):
  - `config set openrouter.max-cost 0.5` writes `max-cost = 0.5` under `[openrouter]`, creating the table and keeping other keys and comments;
  - `config unset typesafe.model` removes the key and then the table once empty;
  - `config get openrouter.model` resolves as if OpenRouter were active, while `config get model` follows the active provider;
  - `config list` shows the active provider's values with origins `config file`, `config file [openrouter]` and `default`, a section value beating the top level;
  - rejected with exit 2 and the file unchanged: `typesafe.max-cost` (naming `typesafe.max-tokens`), `openrouter.provider`, `anthropic.model`, and a file with an `[anthropic]` table or a `provider` inside a table (on load, naming the file)
- [X] T025 [P] [US4] Extend `tests/pipeline/limits.rs` and `tests/pipeline/providers.rs` (US4 scenarios 1–3, SC-006):
  - `openrouter.max-cost` stops an OpenRouter run and is ignored on TypeSafe;
  - `typesafe.max-tokens` stops a TypeSafe run and is ignored on OpenRouter;
  - `openrouter.model` and `typesafe.model` are sent to their own stand-in;
  - a top-level `max-cost = 0.5` on TypeSafe fails with the move message naming `jevpipe config set openrouter.max-cost 0.5` and `jevpipe config unset max-cost` (exit 2, no request), while `--max-cost 0.5` alone still gets the flag message

### Implementation for User Story 4

- [X] T026 [US4] Scoped keys in `src/config/keys.rs` (R9, data-model.md Config): `enum Scope { Every, Only(Provider) }`; `ScopedKey { scope, key }` parsed from `max-cost` or `<provider>.max-cost`; validation per scope (`provider` only at `Every`, `max-cost` not at `Only(TypeSafe)` with `TypeSafe reports no cost; use typesafe.max-tokens`); the unknown-key message lists the keys and says each also works as `openrouter.<key>` or `typesafe.<key>`
- [X] T027 [US4] Tables in `src/config/file.rs`: `read` yields entries from the top level and from `openrouter`/`typesafe` tables (standard or dotted), rejecting any other table; `set`/`unset` take a `ScopedKey`, `set` creating the table (`toml_edit::Table`, keeping decor) and `unset` removing a table left empty; comments and other keys preserved
- [X] T028 [US4] Merged view in `src/config/mod.rs` (point 5): `Config` keeps validated `(ScopedKey, value)` entries; `view(provider)` gives the table's entries over the top level; `settings()`, `setting(key)` and `base_url()` read `view(self.provider())`; `setting` accepts a `ScopedKey` so `config get openrouter.model` resolves for that provider; `Origin::ConfigFile(Scope)` displays `config file` or `config file [<provider>]`; `shared_spend_limit()` returns the top-level `max-cost`
- [X] T029 [US4] `src/config/command.rs`: `get`, `set` and `unset` parse `ScopedKey`; `list` prints origins with their scope
- [X] T030 [US4] The move message in `src/pipeline.rs` (data-model.md Run start check): when `config.shared_spend_limit()` is `Some(V)`, `SpendLimitUnsupported` reads `max-cost = V in the config file applies to every provider, but TypeSafe reports no cost; keep it for OpenRouter with jevpipe config set openrouter.max-cost V and jevpipe config unset max-cost, and limit TypeSafe runs with max-tokens`; otherwise T022's flag message; update T019's config-case expectation to this message
- [X] T031 [US4] `config --help` in `src/cli.rs` explains provider sections and the `<provider>.<key>` form with one example; update `tests/cli/help.rs`

**Checkpoint**: all story tests pass.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T032 [P] Update `README.md` (R11, FR-016; all links absolute): the install section says the key is an OpenRouter or TypeSafe key, links both key pages (`https://openrouter.ai/settings/keys`, `https://console.typesafe.ai/keys`) and shows `jevpipe config set provider typesafe`; "Cost and speed" covers billing on either provider, `--max-tokens`, that `--max-cost` needs OpenRouter, and a two-line example of provider sections; the license note says "through OpenRouter or TypeSafe's API"
- [X] T033 [P] Update `skills/jevpipe/SKILL.md`: the `compatibility` line names OpenRouter or TypeSafe; `--max-tokens` next to `--max-cost` (the `tests/cli/docs.rs` check requires every help flag to be documented)
- [X] T034 [P] Update `CLAUDE.md`: the API key rule names `OPENROUTER_API_KEY` or `TYPESAFE_API_KEY` by provider; the intro says Jev is reached through OpenRouter or TypeSafe's own API
- [X] T035 Grep `src/`, `tests/`, `README.md`, `skills/` for `~typesafe/jev-latest`, `OPENROUTER_API_KEY` and `OpenRouter` and fix any text that should now name both providers
- [X] T036 Run `./check.ps1 -Fix` (format, clippy, tests, deny) with `pwsh` and fix everything
- [X] T037 Run [quickstart.md](quickstart.md) against the real services with the maintainer's keys (well under a cent), with a throwaway `JEVPIPE_CONFIG`; record anything that differs from the contracts

---

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** → user stories.
- **US1** needs Phase 2. **US2** needs Phase 2 (T006, T007) and uses T008's TypeSafe stand-in; it
  does not need US1's code. **US3** needs Phase 2; T022 needs `Config::provider()` (T004). **US4**
  needs US1 (provider in force), US3 (T022's refusal, which T030 extends).
- **Polish** after the stories it documents; T036 last before T037.
- Within a story: tests first (they fail), then implementation in the listed order.

### Parallel Opportunities

- T002 and T003 (new files).
- US1: T009, T010, T011 (different test files); T015 alongside T012–T014.
- US2 and US3 can proceed in parallel after Phase 2 (different files except `src/cli.rs` help,
  done last in each).
- US3: T019 and T020.
- US4: T024 and T025.
- Polish: T032, T033, T034.

## Implementation Strategy

1. **Foundation first**: Phases 1–2 add the types, the provider key, tokens in the reply, the meter
   refactor and the stand-in's TypeSafe mode; every existing test still passes.
2. **MVP**: US1. Stop and verify: a TypeSafe key runs `filter` end to end (quickstart §1). This alone
   answers the Show HN request.
3. **Increment**: US2 (summary), US3 (token limit and the refusal), US4 (provider sections).
4. **Finish**: docs, `check.ps1`, the quickstart with real keys; release 0.2.0 separately.

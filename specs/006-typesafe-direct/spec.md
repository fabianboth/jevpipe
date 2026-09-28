# Feature Specification: TypeSafe's Own API as a Second Provider

**Feature Branch**: `006-typesafe-direct`
**Created**: 2026-09-28
**Status**: Draft
**Input**: User description: "006 Direct TypeSafe API support: jevpipe works with a TypeSafe key against TypeSafe's own API (https://api.typesafe.ai/v1/systemone, model jev-latest), not only through OpenRouter. Open points from the user: (1) keys — today only OPENROUTER_API_KEY or the keychain entry exist; the key may need renaming or a second key for TypeSafe; (2) spending limits — TypeSafe replies report usage tokens (input_tokens, output_tokens) but no cost, so --max-cost cannot work there: either limit by --max-tokens or state that limiting is not supported; (3) the run summary cannot show dollars for TypeSafe: show tokens instead, or a placeholder like ---$. Current behaviour (checked in the code): a reply without cost parses fine (cost None), the summary omits dollars, and only --max-cost fails with NoCost. Motivation: an HN user (cs1996) made a typesafe.ai account and put down $5, and should be able to use that credit with jevpipe. Wire format must come from the official TypeSafe docs (docs.typesafe.ai/api.md)."

## Context

jevpipe reaches Jev through OpenRouter only (milestone 003 left TypeSafe's own endpoint out of scope,
because it uses other model names and reports no cost). People who sign up at typesafe.ai and buy
credit there hold a TypeSafe key, not an OpenRouter key, and today cannot use it with jevpipe. The
first user to ask did so on the Show HN thread, after putting $5 on a TypeSafe account.

TypeSafe's own API speaks the same request format on the same path as OpenRouter's System One API.
Three things differ: the address (`https://api.typesafe.ai`), the names of pinned versions
(`jev-1.13.0` on TypeSafe, `jev-1.13` on OpenRouter; `jev-latest` works on both), the usage report,
which counts input and output tokens but carries no cost, and the shape of error replies. TypeSafe publishes a price per input token (Jev 1.13: $0.042 per million, output tokens
free), so tokens are the unit its credit is spent in.

This milestone makes TypeSafe a second provider next to OpenRouter. The user chooses the provider
once, gives it its own key, and the run options, limits and summary work on both, with tokens as the
measure that both providers report.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run jevpipe on a TypeSafe key (Priority: P1)

A developer with a TypeSafe account sets the provider once (`jevpipe config set provider typesafe`)
and provides the TypeSafe key, either as `TYPESAFE_API_KEY` in the environment or stored once with
`jevpipe auth set-key`. From then on `filter` and `map` run against TypeSafe's API with Jev's current
model and spend that account's credit. Nothing else about the pipeline changes.

**Why this priority**: It is the feature: without it a TypeSafe key cannot be used at all.

**Independent Test**: With the provider set to TypeSafe and `base-url` pointing at a local stand-in
that answers like TypeSafe (tokens, no cost), run `filter` and `map` over a few lines with
`TYPESAFE_API_KEY` set; the stand-in receives the TypeSafe key and the TypeSafe model name, the
outputs equal those of the same run on OpenRouter's stand-in, and the exit status is 0.

**Acceptance Scenarios**:

1. **Given** `provider` set to `typesafe` and `TYPESAFE_API_KEY` set, **When** `filter` runs, **Then** each request goes to TypeSafe's address with that key and the model `jev-latest`, and the kept lines are printed as with OpenRouter.
2. **Given** `provider` set to `typesafe`, no `TYPESAFE_API_KEY` and a TypeSafe key stored with `auth set-key`, **When** a run starts, **Then** the stored TypeSafe key is used.
3. **Given** `provider` set to `typesafe` and only `OPENROUTER_API_KEY` set (or only an OpenRouter key stored), **When** a run starts, **Then** it fails before reading input with a message naming `TYPESAFE_API_KEY` and `jevpipe auth set-key`, exit status 2; the OpenRouter key is never sent to TypeSafe.
4. **Given** no provider configured, **When** a run starts, **Then** OpenRouter is used as in 0.1.1: the same address, key variable and stored key, and the default model `jev-latest` resolves to the same model as 0.1.1's `~typesafe/jev-latest`.
5. **Given** the default provider and only `TYPESAFE_API_KEY` set, **When** a run starts, **Then** it fails with a message naming the provider in use, how to give its key, and how to switch provider (`jevpipe config set provider typesafe`).
6. **Given** `provider` set to `typesafe` and `--model jev-1.13.0`, **When** a run starts, **Then** that model is asked; the model option works the same on both providers, only its default differs.
7. **Given** `provider` set to `typesafe`, **When** the user runs `jevpipe config list`, **Then** it shows the provider, the effective address and model for it, and where the TypeSafe key comes from (environment, keychain, or not set), without the key.
8. **Given** a TypeSafe key that the service rejects, or a model name it does not know (such as OpenRouter's `jev-1.13`), **When** a run starts, **Then** it stops with the service's own message (for example `Unknown model: jev-1.13`), exit status 2, as for OpenRouter.

---

### User Story 2 - See what a run used on either provider (Priority: P2)

A developer or agent reads the one-line summary on standard error after a run and sees what the run
consumed, whichever provider it ran on: the tokens used (both providers report them) and the
cost in dollars where the service reports it (OpenRouter).

**Why this priority**: Without it a TypeSafe run gives no sense of its size, and the account's credit
drains without feedback; but runs work without it.

**Independent Test**: Run the same `map` over 10 lines against a stand-in that reports tokens without
cost and against one that reports both; both summaries show the token total, the second also the
dollar cost, and both show the counts and time as before.

**Acceptance Scenarios**:

1. **Given** a service that reports tokens and no cost, **When** a run ends, **Then** the summary shows the total tokens reported (input plus output), for example `100 records, 12 kept, 0 skipped, 0 failed, 41.3k tokens, 2.1s`.
2. **Given** a service that reports tokens and cost, **When** a run ends, **Then** the summary shows both, for example `100 records, 12 kept, 0 skipped, 0 failed, 41.3k tokens, $0.0017, 2.1s`.
3. **Given** a service that reports neither, **When** a run ends, **Then** the summary shows neither, as today; each measure is shown exactly when some answer reported it.
4. **Given** a run in which some answers report cost and some do not, **When** it ends, **Then** the summary shows the dollars reported, and does not present them as the whole run's cost without saying so.

---

### User Story 3 - Limit a run by tokens (Priority: P3)

A developer or agent adds `--max-tokens 5M` (or sets `max-tokens` in the config file) so that one run
can never consume more than that many tokens. It works on both providers, and on TypeSafe it is the
way to cap spend, since tokens are what TypeSafe bills. It stops the run exactly like `--max-cost`:
the prefix of decided output, one message naming the limit and the resume line, exit status 3.

**Why this priority**: A safety net for unattended runs on TypeSafe, where `--max-cost` cannot work;
the account's own credit balance still bounds the worst case, so it is not needed for the first use.

**Independent Test**: Against a stand-in that reports 330 tokens per answer and no cost, run `map`
over 100 lines with `--max-tokens 3300` and `--concurrency 1`; the output holds the first 10 answered
lines, standard error names the token limit and line 11 as the resume line, and the exit status is 3.

**Acceptance Scenarios**:

1. **Given** `--max-tokens 3300`, a stand-in reporting 330 tokens per answer and `--concurrency 1`, **When** `map` runs over 100 lines, **Then** no request is sent once the reported total reaches 3,300 tokens, the output holds exactly the lines decided before that point, standard error holds `jevpipe: stopped: token limit 3300 reached (3300 used); input from line 11 on was not processed`, and the exit status is 3.
2. **Given** `--max-tokens` and several requests in flight when it is reached, **When** they complete, **Then** they are handled as for `--max-cost` (output if every earlier record was, counted in the total, nothing further sent).
3. **Given** `--max-tokens` on OpenRouter, **When** the run reaches it, **Then** it stops the same way; `--max-cost` and `--max-tokens` may both be set, and whichever is reached first stops the run.
4. **Given** `--max-tokens` and an answer that reports no tokens, **When** it arrives, **Then** the run stops with a run-level error saying the token limit cannot be enforced, exit status 2 (as for `--max-cost` without cost).
5. **Given** `provider` set to `typesafe` and a spend limit from the command line or the config file's top level, **When** a run starts, **Then** it fails before any request with a message saying TypeSafe reports no cost and pointing to `--max-tokens` (and, for the config file, to `openrouter.max-cost`), exit status 2; `--max-cost none` lifts it for one run.
6. **Given** a value such as `5M`, `250k` or `1000000`, **When** it is given as `--max-tokens` or `config set max-tokens`, **Then** it is accepted; `none` lifts a configured limit for one run; zero, negative, fractional or other values are usage errors before any input is read.

---

### User Story 4 - Keep settings per provider (Priority: P4)

A developer who uses both providers keeps the settings that only make sense for one of them in that
provider's section of the config file: a spend limit and a pinned model for OpenRouter, a token limit
and TypeSafe's pinned model for TypeSafe. Switching the provider then switches those settings with
it, and nothing has to be unset or set again.

```sh
jevpipe config set openrouter.max-cost 0.5
jevpipe config set openrouter.model jev-1.13
jevpipe config set typesafe.model jev-1.13.0
jevpipe config set typesafe.max-tokens 5M
```

**Why this priority**: Only people switching between providers need it; a single-provider setup
works with the plain keys. But without it, a spend guard or a pinned model set for one provider
blocks or breaks runs on the other.

**Independent Test**: With `openrouter.max-cost 0.001` and `typesafe.max-tokens 3300` in the config,
run `map` against the OpenRouter stand-in and then, after `config set provider typesafe`, against
the TypeSafe stand-in; the first run stops at the spend limit, the second at the token limit, and
neither needs a flag or a config change besides the provider.

**Acceptance Scenarios**:

1. **Given** `openrouter.model = "jev-1.13"` and `typesafe.model = "jev-1.13.0"`, **When** a run starts, **Then** the active provider's model is asked; after `config set provider` the other one is.
2. **Given** a key both at the top level and in the active provider's section, **When** a run starts, **Then** the section's value applies; a flag still beats both.
3. **Given** a key only in the other provider's section, **When** a run starts, **Then** it does not apply.
4. **Given** `config set typesafe.max-cost 1`, **When** it runs, **Then** it is rejected because TypeSafe reports no cost, naming `typesafe.max-tokens`, and the file is unchanged.
5. **Given** `config set openrouter.provider typesafe` or a section other than `openrouter` and `typesafe`, **When** it runs or the file is loaded, **Then** it is rejected naming the valid keys.
6. **Given** sections in the file, **When** the user runs `config list`, **Then** each key shows the value that applies to the active provider and where it comes from (`config file`, `config file [openrouter]`, or `default`).
7. **Given** `config get openrouter.model`, **When** it runs, **Then** it prints the value that would apply with OpenRouter active; `config get model` prints the one for the active provider.
8. **Given** `config unset typesafe.model` removes the last key of its section, **When** it runs, **Then** the empty section is removed too; other keys and comments are preserved.

---

### Edge Cases

- **A spend limit at the top level and the provider TypeSafe**: every run fails before any request, and the message shows how to keep the limit for OpenRouter only (`jevpipe config set openrouter.max-cost <value>` and `jevpipe config unset max-cost`) and to use `max-tokens` for TypeSafe. A spend guard is never silently dropped.
- **`--max-cost` on the command line with the provider TypeSafe**: fails before any request, pointing to `--max-tokens`.
- **A configured model and a switch of provider**: a model at the top level applies to both providers; a pinned name valid on one (`jev-1.13` on OpenRouter, `jev-1.13.0` on TypeSafe) is rejected by the other with the service's message, so pinned models belong in the provider's section. The default `jev-latest` works on both.
- **A `base-url` configured together with a provider**: `base-url` overrides the provider's address (for gateways and the test stand-in); the provider still decides the key, the default model and how usage is read.
- **Both keys present**: only the configured provider's key is used; the other is never read or sent.
- **TypeSafe's credit runs out**: TypeSafe documents no error for an empty balance; the service's refusal is a run-level error with its message, exit status 2. Should TypeSafe document a machine-readable "credits used up" error, it becomes a limit stop (exit status 3) like OpenRouter's.
- **Token counts that are missing or malformed in one answer**: the answer itself is still used; only a `--max-tokens` run stops, as in User Story 3, scenario 4; a malformed count (not a whole non-negative number) is a failed record with a message, as for a malformed cost.
- **Very large token totals**: the summary abbreviates them (`41.3k`, `12.5M`) so the line stays short; the stop message shows exact numbers.
- **An unknown provider name** in the config file or `config set provider`: rejected with the valid names, as for other invalid values.
- **Stored OpenRouter keys from 0.1.x**: keep working unchanged, without being stored again.

## Requirements *(mandatory)*

### Functional Requirements

**Providers**

- **FR-001**: jevpipe MUST support two providers, `openrouter` (default) and `typesafe`, chosen by the `provider` config key. The provider is a setup choice, set once, not a per-run flag.
- **FR-002**: Each provider MUST define the default service address (`https://openrouter.ai/api` and `https://api.typesafe.ai`), the environment variable for its key (`OPENROUTER_API_KEY` and `TYPESAFE_API_KEY`, the name TypeSafe's own SDKs use), and its own stored key. The default model MUST be `jev-latest` on both.
- **FR-003**: `base-url` and `model`, when set, MUST override the provider's defaults; the effective values MUST be shown by `config list`, `config get` and `--help`.

**Per-provider settings**

- **FR-003a**: The config file MUST accept a section per provider (`[openrouter]`, `[typesafe]`) holding any config key except `provider`. For a run, a flag beats the active provider's section, which beats the top level, which beats the built-in default; the other provider's section is ignored.
- **FR-003b**: `config set`, `config unset` and `config get` MUST accept a key prefixed with a provider (`openrouter.max-cost`), validated like the plain key; `config set` creates the section when missing and `config unset` removes it when it becomes empty. `config get <provider>.<key>` prints the value that applies with that provider active.
- **FR-003c**: `config list` MUST show the values that apply to the active provider, each with its origin: `config file`, `config file [<provider>]` or `default`.
- **FR-003d**: A spend limit in the `typesafe` section MUST be rejected (file load and `config set`), naming `typesafe.max-tokens`; an unknown section or a `provider` key inside a section MUST be rejected naming the valid keys.
- **FR-004**: The request and response format for TypeSafe MUST follow TypeSafe's published API reference; for OpenRouter nothing changes.
- **FR-005**: With no `provider` configured, every behaviour of 0.1.1 MUST be unchanged, including stored OpenRouter keys, except the spelling of the default model (`jev-latest`, the same model), the token total in the summary, and the missing-key message naming the provider.
- **FR-005a**: A service's refusal MUST be reported with the service's own message on both providers (OpenRouter's and TypeSafe's error replies differ in shape).

**API keys**

- **FR-006**: A run MUST use the configured provider's environment variable when it is set and non-empty, otherwise that provider's stored key, otherwise fail before reading input with a message naming both ways to provide it. It MUST never send one provider's key to the other.
- **FR-007**: `auth set-key` and `auth remove-key` MUST act on the configured provider's key and name the provider in their messages; all other rules for storing keys (hidden prompt, validation, no key on the command line, no confirmation dialog after updates) stay as in milestone 003.
- **FR-008**: The missing-key message MUST name the provider in use, both ways to give its key, and the `config set provider` command that switches to the other provider. It does not depend on which other variables are set.
- **FR-009**: `config list` MUST show where the configured provider's key comes from, without the key.

**Usage and summary**

- **FR-010**: jevpipe MUST read the input and output token counts from each answer's usage report where present, on both providers, in addition to the cost where present.
- **FR-011**: The run summary MUST show the total reported tokens (input plus output, abbreviated with k/M) when any answer reported them, and the reported cost in dollars when any answer reported one, each independently. No placeholder such as `---$` is shown.

**Token limit**

- **FR-012**: `filter` and `map` MUST accept `--max-tokens <COUNT|none>` and the config key `max-tokens`: once the total reported tokens of this invocation reach the limit, no further request is sent; requests in flight complete. There is no default limit. Counts accept a number with an optional `k` or `M` suffix (`250k`, `5M`, `1.5M`).
- **FR-013**: A run stopped by the token limit MUST behave as one stopped by `--max-cost` (prefix output, one stop line naming the limit and the tokens used, resume line, exit status 3).
- **FR-014**: With `--max-tokens` set, an answer without reported tokens MUST stop the run with a run-level error saying the limit cannot be enforced, exit status 2.
- **FR-015**: With the provider `typesafe`, a spend limit (from the command line or the config file's top level) MUST fail the run before any request, exit status 2. The message MUST name `--max-tokens`; for a limit from the config file it MUST also give the commands that move it to `openrouter.max-cost`.

**Help, docs and verification**

- **FR-016**: `--help`, the README and the agent skill MUST describe the providers, how to switch to TypeSafe, both key variables, `--max-tokens`, per-provider settings, and that `--max-cost` needs a provider that reports cost.
- **FR-017**: All behaviour MUST be verifiable by automated tests without network access, API keys, the real keychain or the developer's config, using the local stand-in in a TypeSafe mode (tokens, no cost) next to its OpenRouter mode, as the project's testing rule in CLAUDE.md requires.

### Key Entities

- **Provider**: which service jevpipe talks to; defines default address, key variable, stored key, and whether answers report cost.
- **Config scope**: the top level of the config file (every provider) or one provider's section; a setting's origin names it.
- **Usage**: what one answer consumed: input and output tokens, and cost where the provider reports it.
- **Run limits**: the existing spend and time limits plus a token limit, per invocation.
- **Stored API key**: one per provider in the operating system's keychain; the provider's environment variable takes precedence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with only a TypeSafe key gets a first successful `filter` run with at most two setup commands (choose the provider, provide the key).
- **SC-002**: Every existing 0.1.1 behaviour test passes apart from the three changes FR-005 names: users of OpenRouter see no difference in output, limits or exit status.
- **SC-003**: With a token limit, the reported tokens of a run never exceed the limit by more than the tokens of the requests in flight when it was reached, in 100% of test scenarios.
- **SC-004**: Every run on either provider ends with a summary that states what it consumed (tokens, and dollars where reported) whenever the service reported it.
- **SC-005**: No test scenario sends a key to a provider other than the one it belongs to, or shows a key in any output, file or command line.
- **SC-006**: A user with a spend limit and a pinned model for each provider switches providers with one command (`config set provider`) and no other config change.
- **SC-007**: The full automated test suite passes on Linux, Windows and macOS without network, API key, keychain access or user config.

## Assumptions

- TypeSafe's API reference (docs.typesafe.ai/api.md) is the source for the format: `POST /v1/systemone`, bearer key, the same `state`, `model` and `questions` fields, answers in the same shape, and `usage` with `input_tokens` and `output_tokens` only. TypeSafe's SDKs read the key from `TYPESAFE_API_KEY` and default to `https://api.typesafe.ai` and `jev-latest`.
- The documented format was confirmed with live requests on both providers during planning (research R1): identical answers, `usage` with tokens only on TypeSafe, bare `jev-latest` accepted by both, and TypeSafe's own error shape. The product and its tests never depend on a real key.
- OpenRouter's usage report for Jev carries input and output tokens (required in its API schema) next to the optional cost, so tokens are the common measure of both providers.
- A token limit is used in place of a dollar estimate computed from TypeSafe's published price: the price changes with model releases and would have to be kept current in jevpipe, while the token count is exactly what the service reports. jevpipe states no price anywhere, so nothing in it goes stale when TypeSafe changes its prices.
- Tokens are counted as input plus output, the total both providers report; on TypeSafe output tokens are free and few (one short answer per question), so the total tracks the billed amount closely.
- The provider is a config key only, like `base-url`, because it goes together with a key and is chosen once per machine or environment; per-run switching is out of scope.
- Provider sections use TOML tables and dotted keys, so `config set openrouter.max-cost 0.5` writes the same `max-cost` under `[openrouter]` that a user would type by hand. Everything after loading sees one merged list of settings for the active provider, so the flags, `--help` defaults and the runs themselves do not change.
- The existing stored key keeps its keychain entry, so 0.1.x users need not store it again; the TypeSafe key gets its own entry next to it.
- The provider, the new key variable, provider sections and `--max-tokens` are additions; for OpenRouter users only the three changes FR-005 names are visible. The release is 0.2.0 because it adds features.

## Out of Scope

- Other providers than OpenRouter and TypeSafe, and gateways that speak a different format.
- Dollar estimates from price tables, querying the account balance, and budgets across invocations.
- Choosing the provider per run with a flag, or detecting it from which key happens to be set.
- A limit stop for an exhausted TypeSafe balance until TypeSafe documents a machine-readable error for it.

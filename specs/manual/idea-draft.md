# jevpipe

A Unix pipe for typed decisions. Stream records in, get calibrated decisions out, from Jev (TypeSafe's System One model) or any server speaking the same API.

## The idea

Coding agents are good at planning and writing small scripts, and bad at reading thousands of lines or making hundreds of tiny judgments. System One models like Jev are the opposite: they can't generate or explore, but they answer typed questions (yes/no, pick one, score) with calibrated probabilities in milliseconds, for fractions of a cent.

So the agent writes the loop and `jevpipe` makes the decisions inside it. The agent's script gathers state and lists the options, `jevpipe` decides, and the script acts. Only the outcome reaches the agent's context. It's a reflex layer: much cheaper and faster than a subagent, and used on the fly for throwaway work.

## What it should feel like

```bash
# semantic grep: find code by what it does
git ls-files | jevpipe filter "handles player input buffering" --read-files

# check a diff for scope creep
git diff | jevpipe filter "a change unrelated to increasing the buffer size"

# label a stream of records
cat failures.jsonl | jevpipe map --choice flaky="infra or timing" real="deterministic bug"

# interactive loop: a long-running process driven by any script
# (browser, game engine, test runner), one decision per line
jevpipe serve --choice-from options --noul done="Is the goal reached?"
```

Records are plain lines or JSONL. Output is JSONL that keeps each record's id, so it composes with `jq`, `head`, `xargs` and friends. In interactive loops the candidate options change every step, so each record can carry its own options.

## Principles

- **Decide, don't act.** No planning, no text generation, no executing actions. The calling script owns all of that.
- **Stateless.** For multi-turn loops, the driver keeps the history and sends the current state each step.
- **Cheap to rerun.** Cache identical requests, so agents re-running a pipeline pay nothing.
- **Safe to run unattended.** Budgets on records, spend and time. Clean exit codes. A one-line summary on stderr.
- **Fast.** Near-zero startup, efficient batching and concurrency for bulk input, low latency in interactive mode.
- **JEV-compatible.** connect to the TypeSafe API on OpenRouter. (maybe more agnostic)

## Constraints

- Rust, shipped as a single binary.
- Take the wire format from the official TypeSafe API docs.

## Deliverables

1. The `jevpipe` CLI.
2. An agent skill (`skills/jevpipe/SKILL.md`) that teaches coding agents when to reach for it (bulk or repetitive judgments, enumerable step loops), when not to (exact grep, arithmetic, anything needing generated text), how to phrase questions, and to treat review items as their own.
3. Examples: semantic grep, diff scope check, record triage, and a small browser loop driven by a Playwright script.

## First milestone

`filter` working end to end on files and stdin, with offline tests. Stop there for review before building `map` and `serve`.

## Later

Deferred from the first milestone (`specs/001-filter-foundation`): `map` and `serve`, caching identical requests, budgets on records, cost and run time, inverted matching (`-v`), sorting by probability, other providers (TypeSafe directly, a shared key name), JSON records as structured state (when `map` needs fields such as `id`), and the agent skill with the example pipelines.

Learnings for the skill so far: [skill-learnings.md](skill-learnings.md).

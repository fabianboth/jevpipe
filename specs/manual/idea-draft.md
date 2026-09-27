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

# label a stream of records: several typed questions in one request per record
jevpipe map -f triage.json failures.jsonl | jq -c 'select(.answers.kind.choice == "flaky") | .record'

# interactive loop (browser, game engine, test runner): one `map` per step,
# with that step's questions inline
echo "$PAGE_STATE" | jevpipe map -q "$STEP_QUESTIONS"
```

Records are lines (plain text or JSONL, sent as text) or file paths. `filter` prints the kept lines unchanged, so it composes with `head`, `xargs` and `wc`; `map` prints one JSON line per record with the record and its answers, for `jq`.

## Principles

- **Decide, don't act.** No planning, no text generation, no executing actions. The calling script owns all of that.
- **Stateless.** For multi-turn loops, the driver keeps the history and sends the current state each step.
- **Cheap to rerun.** A run stopped by a limit names the input line to resume from, so a rerun pays only for the rest.
- **Safe to run unattended.** Per-run limits on spend and time (`--max-cost`, `--max-time`); OpenRouter's key limit and credits stop a run the same way. Clean exit codes. A one-line summary on stderr.
- **Fast.** Near-zero startup, efficient batching and concurrency for bulk input, low latency per step in a loop.
- **JEV-compatible.** Connects to the TypeSafe API on OpenRouter, and only there. Defaults live in a user config file (`jevpipe config`), the API key in `OPENROUTER_API_KEY` or the system keychain (`jevpipe auth set-key`).

## Constraints

- Rust, shipped as a single binary.
- Take the wire format from the official TypeSafe API docs.

## Deliverables

1. The `jevpipe` CLI.
2. An agent skill (`skills/jevpipe/SKILL.md`) that teaches coding agents when to reach for it (bulk or repetitive judgments, enumerable step loops), when not to (exact grep, arithmetic, anything needing generated text), how to phrase questions, and to treat review items as their own.
3. Examples: semantic grep, diff scope check, record triage, and a small browser loop driven by a Playwright script.

## Next

1. **004 Release and skill.** Prebuilt binaries for Linux, macOS and Windows (x64 and arm64) via
   cargo-dist on a version tag: GitHub Releases with shell and PowerShell install scripts, plus npm
   (`jevpipe`, reserved with a 0.0.0 placeholder by the npm account `bothlabs`; published from CI through npm
   trusted publishing, no stored token). The skill `skills/jevpipe/SKILL.md` is written from
   what is known so far and installs separately through `npx skills add` or `gh skill install`; it
   carries no binary and no install steps, only a pointer to the README when `jevpipe` is missing.
   The README covers installation and the API key. Tried on the private repo with a pre-release tag.
2. **005 Calibration study.** Labeled gold-standard sets per use case; precision and recall per
   threshold, calibration, run time, throughput and cost per 1,000 records, against grep and a
   general LLM as baselines; trigger and task evals for the skill (developer tooling in the repo,
   not shipped, run by hand). Results set the defaults, revise the skill, and go into the README
   with date and model version. The example pipelines (deliverable 3) come out of these use cases.
3. The repo goes public at the end of 004, with a README, a license and a first release; 005 adds
   the numbers behind "calibrated".

## Later

Deferred: budgets on records, inverted matching (`-v`), sorting by probability, per-record questions for a long-running step loop, and JSON records as structured state (if a measurement shows it helps).

Dropped: caching identical requests (identical requests are rare, and provider-side input caching is not live for Jev) and providers other than OpenRouter.

Learnings for the skill so far: [skill-learnings.md](skill-learnings.md).

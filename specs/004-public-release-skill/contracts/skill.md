# Contract: Agent Skill

Location: `skills/jevpipe/SKILL.md`, the only file in `skills/jevpipe/` (text, under 50 KB). No
`SKILL.md` at the repository root. Written with the skill-creator guidance (research R7).

## Frontmatter

```yaml
---
name: jevpipe
description: <third person, 1–1024 characters, "a little bit pushy">
license: Apache-2.0
compatibility: Requires the jevpipe CLI (0.1.0) on PATH and network access to OpenRouter; jq recommended for map.
metadata:
  jevpipe-version: "0.1.0"
---
```

No `allowed-tools`: every run spends the user's credit, so the user keeps approving runs.

The description says what jevpipe does (filter as a semantic grep, map for several typed questions
as JSON lines) and when to use it even if jevpipe is not named: the same judgment repeated over dozens
to thousands of items that no exact pattern captures (finding files by what the code does; triaging
log lines, test failures, issues or diff hunks; labelling or scoring a list; deciding each step of a
scripted loop). It names what it is not for: exact text search, counting, arithmetic, generated text.

## Body sections (imperative, examples, reasons instead of capitalised rules; ~150–220 lines)

1. **What it is** – jevpipe decides, the agent acts; only the outcome reaches the context. Written
   for jevpipe 0.1.0; when a flag is rejected, check `jevpipe <command> --help`. (FR-013, FR-018)
2. **When to use it, when not** – many independent judgments, concepts without a keyword, step
   loops; not exact search (grep/rg), counts or maths, generated text. (FR-013)
3. **Before the first run** – `jevpipe` missing: stop and tell the user it needs installing, link
   https://github.com/fabianboth/jevpipe; do not build or install it. No key (the error names
   `OPENROUTER_API_KEY` and `jevpipe auth set-key`): ask the user to do one of them themselves;
   never ask for, read, print or pass the key, and never run `auth set-key` (it reads the key from
   the user), with the reason. (FR-016)
4. **filter or map** – one yes/no question → `filter`; several questions or choice/score answers →
   `map`. Inputs: `git ls-files`, `rg --files`, `find` with `--read-files`; lines from logs or
   commands. Examples with `--max-cost`. (FR-014, FR-014a)
5. **Phrasing questions** – about the record itself, specific; broad topics also match documents
   about the topic. (FR-014)
6. **Thresholds and the review band** – 0.5 when both mistakes cost the same, higher when acting on a
   false yes is expensive, a middle band (e.g. 0.2–0.8) as the agent's own review items; pin
   `--model typesafe/jev-1.13` for repeatable runs. Marked as current guidance, revisited after
   calibration. (FR-014, FR-017, FR-018)
7. **map output with jq** – the answer shapes (`noul`, `choice`, `score`) and three jq usages:
   `select(.answers.x.noul >= 0.8) | .record`, `select(.answers.kind.choice == "flaky")`,
   `select(.outcome == "failed")`. No fallback, no jq install steps. (FR-014a)
8. **Limits, exit status, resuming** – set `--max-cost` for every run over more than a handful of
   records, `--max-time` when a deadline matters; exit statuses (filter 0 kept / 1 none kept /
   2 failed / 3 limit; map 0 / 2 / 3); the one-line summary on standard error; on 3, resume from the
   named line (`tail -n +N`), do not rerun blindly with a higher limit. (FR-015)

## Checked automatically (`tests/cli/docs.rs`)

- `name` equals the folder name.
- Every `--flag` in the body exists in the binary's help.

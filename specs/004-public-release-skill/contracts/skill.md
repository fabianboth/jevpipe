# Contract: Agent Skill

Location: `skills/jevpipe/SKILL.md`, the only file in `skills/jevpipe/` (text, under 50 KB). No
`SKILL.md` at the repository root. Written with the skill-creator guidance (research R7) and reworked
with the maintainer after the README (less prose, real outputs, TypeSafe's own guidance).

## Frontmatter

```yaml
---
name: jevpipe
description: <what it is (bulk System 1 decisions from a configurable decision model), when to use it (the same judgment over many items, even if jevpipe is not named, with concrete jobs), its limits (each item alone, answers not text); well under 1024 characters>
license: Apache-2.0
compatibility: Requires the jevpipe CLI on PATH and network access to OpenRouter; jq recommended for map.
---
```

No version in the skill, no `allowed-tools`.

## Body sections, in this order

1. **Opening** – one paragraph: a System 1 the agent programs from the shell; typed answers with
   their confidence; seconds and a fraction of a cent; none of the items pass through the agent's
   context.
2. **When to use it** – the moment to notice (going through many items one by one) and three
   criteria: each item judged from its own content (first 100,000 characters), a quick judgment,
   enough items.
3. **How to use it** – the pipe pattern for `filter` and `map`; input like grep (lines piped or from
   named files, `--read-files` for whole files); output (`filter` passes lines on, `map` prints JSON
   lines).
4. **filter** – the real ripgrep 15.1 example and its output.
5. **map** – the release-notes `choice` example, one output line, the three answer types, records
   without an answer, jq piped directly or on a saved file when sliced several ways.
6. **Writing questions** – say exactly what you mean (a topic also matches files that only discuss
   it); TypeSafe's advice per type, linked to their primitives page.
7. **Reading the answers** – TypeSafe's guidance, linked to their confidence page: `noul`
   thresholds by the cost of being wrong and a review band; confidence for `choice` and `score`; a
   score as a probability-weighted position.
8. **Cost** – cost per 1,000 records by size; the user's spending cap stops a run by itself; narrow
   the input by what is certain.
9. **Exit status** – 1 (nothing matched), 2, 3 (cap or time limit, resume line).
10. **Setup and more** – one line linking the README.

## Checked automatically (`tests/cli/docs.rs`)

- `name` equals the folder name.
- Every flag after a `jevpipe` command, and every code span starting with `--` outside code blocks,
  exists in the binary's help.

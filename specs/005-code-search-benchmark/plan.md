# Implementation Plan: Semantic Code Search Benchmark

**Branch**: `005-code-search-benchmark` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/005-code-search-benchmark/spec.md`

## Summary

No product code changes. A benchmark in its own folder, `bench/`, a small typed Python project run as
`uv run bench <stage>`:

1. **Prepare**: the CodeSearchNet Challenge ratings at a pinned commit, the Python snippets from their
   rated commits, a pool of about 940 neutrally named files, a seeded 20/79 dev/test split.
2. **Patterns and wordings**: Codex (GPT-6 Astra) writes one ripgrep pattern per query from the query
   alone; four question wordings are tried on the dev queries and the best is frozen per model.
3. **Run**: per query, the three grep baselines with ripgrep, then jevpipe (`map --read-files`, pinned
   `typesafe/jev-1.13`) and DeepSeek V4.1 Flash (OpenRouter, logprobs) over the whole pool, 100
   requests in flight each, alternating which goes first; five queries run twice.
4. **Judge**: Codex rates every expert-rated pair first (the gate: F1 ≥ 0.67), then every unrated hit
   and two samples, blind and in mixed batches.
5. **Score**: offline, from the committed results: metrics, threshold sweep, the chart and the numbers
   for the method page. Then the README section and `bench/README.md`.

A separate workflow runs the benchmark's offline checks (ruff, pyright strict, pytest) when `bench/`
changes; the product's CI and its required gate are unchanged.

## Technical Context

**Language/Version**: Python 3.14 (`bench/.python-version`); PowerShell 7 for `bench/check.ps1`;
YAML for the CI job
**Primary Dependencies**: `httpx` (async OpenRouter calls), `matplotlib` (chart); dev: `ruff`,
`pyright`, `pytest`; external tools: `jevpipe` 0.1.0, ripgrep, Codex CLI (0.156, `gpt-6-astra`)
**Storage**: JSON files in `bench/results/` (committed), downloads in `bench/.cache/` (gitignored)
**Testing**: pytest on small fixtures for pooling, labels, judge selection, metrics and threshold
sweep (offline); trial run on two queries before the full run
**Target Platform**: the maintainer's machine (Windows 10, also works on Linux and macOS); CI runs the
checks on `ubuntu-latest`
**Project Type**: a standalone script project next to the Rust crate
**Performance Goals**: full pool run in under an hour (spike: about 30 s per query for both models);
`score` in under a minute offline (SC-003)
**Constraints**: OpenRouter spend at most $10 for the whole run (SC-002), under the key's $15 cap;
judging on the user's Codex subscription only; no snippet code or key in the repository; resumable
per query and per batch
**Scale/Scope**: 99 queries × ~940 snippets = ~93,000 decisions per model; ~6,000 judged pairs;
about 10 modules, ~1,000 lines

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass: one dataset, one task, two models and three grep baselines; one chart; stages are plain functions behind argparse; two runtime dependencies; no product changes |
| II. Automated Verification | Pass: `bench/check.ps1` (format, lint, strict types, tests) runs offline, and in its own workflow whenever `bench/` changes; the repository's `check.ps1` is unchanged; paid stages are verified by a two-query trial before the full run |
| III. Reusable Components | Pass: jevpipe is used as released, not re-implemented; one Codex runner serves patterns and judging; one metrics module serves chart, numbers and threshold sweep |

Post-design re-check: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/005-code-search-benchmark/
├── spec.md
├── plan.md              # this file
├── research.md          # decisions and their sources
├── data-model.md        # entities, label and scoring rules
├── quickstart.md        # running the benchmark
├── contracts/
│   ├── cli.md           # the `bench` stages, options, exit status, check script
│   ├── results.md       # stored files and their format
│   └── docs.md          # README section, chart, method page
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
bench/
├── pyproject.toml       # uv project, `bench` script, ruff and pyright config
├── uv.lock
├── .python-version      # 3.14
├── check.ps1            # ruff format, ruff check, pyright, pytest
├── README.md            # the method page
├── src/bench/
│   ├── cli.py           # argparse: one subcommand per stage, exit status
│   ├── limits.py        # LimitReached: spend limit, Codex quota, judge gate → exit 3
│   ├── store.py         # atomic JSON read and write under results/
│   ├── dataset.py       # ratings download, queries, split
│   ├── pool.py          # snippet fetch and cut, neutral names
│   ├── wording.py       # the four wordings, trial and freeze
│   ├── jev.py           # jevpipe subprocess, resolved-version call
│   ├── deepseek.py      # OpenRouter requests, logprob probability
│   ├── runs.py          # per-query pool runs of both models, alternating, resumable
│   ├── grep.py          # keywords, ripgrep runs
│   ├── codex.py         # codex exec runner with output schema
│   ├── patterns.py      # agent-written patterns
│   ├── judge.py         # selection, batches, validation gate
│   ├── labels.py        # expert and judge labels per pair
│   ├── metrics.py       # found, false hits, precision, recall, sweep, bands
│   └── chart.py         # the README chart
├── tests/               # pytest, offline fixtures
└── results/             # committed outputs (contracts/results.md)

.github/workflows/bench.yml # offline checks, only when bench/ changes; not a required check
.gitignore               # + /bench/.cache/, bench virtualenv
README.md                # + the chart section (contracts/docs.md)
CLAUDE.md                # + `bench/` in Layout, its check script in Verification
```

**Structure Decision**: `bench/` is its own uv project beside the crate; nothing under `src/` or
`tests/` changes. Modules follow the concepts of the data model, one each; `cli.py` only wires them.

## Implementation order

1. Skeleton: `pyproject.toml`, `check.ps1`, `bench.yml`; `store`, `dataset`, `pool` with tests; `prepare`.
2. Verify the DeepSeek request shape with `require_parameters` on a handful of snippets (research §4).
3. `codex`, `patterns`, `wording`; run `patterns` and `wordings`.
4. `jev`, `deepseek`, `grep`, the `run` stage; trial on two queries; full run; `repeat`.
5. `labels`, `judge`; validation, then the rest.
6. `metrics`, `chart`, `score`; the method page and the README section; `CLAUDE.md`.

## Complexity Tracking

No violations.

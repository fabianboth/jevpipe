---
description: "Task list for the semantic code search benchmark"
---

# Tasks: Semantic Code Search Benchmark

**Input**: Design documents from `specs/005-code-search-benchmark/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/cli.md](contracts/cli.md),
[contracts/results.md](contracts/results.md), [contracts/docs.md](contracts/docs.md),
[quickstart.md](quickstart.md)

**Tests**: FR-030 requires offline pytest tests of the pooling, label, judge-selection and metrics
rules on small fixtures. No test touches the network, the key or Codex; ripgrep on a temporary folder
is allowed (a local tool). The paid stages are verified by the two-query trial (T027).

**Rules for every task**: the house style of `CLAUDE.md` carried over to Python: no comments (the
`--help` strings of argparse are not comments), one module per concept, frozen dataclasses for
records, at most 3 parameters per function, `match` over enums/literals ends in `assert_never`, no
bare `except` (catch the specific error and re-raise with context), no `# type: ignore`, `# noqa` or
`# pyright:` in code (rule levels live in `bench/pyproject.toml` only), private by default (leading
underscore for module-internal names). Functions that decide are pure and take data; functions that
download, call models or run tools are thin and separate, so the tests need no network.
`bench/check.ps1 -Fix` passes at the end of each phase. The spike in `scratch/bench/` (`spike.py`,
`pool_spike.py`, `judge_proto.py`, `analyze.py`) is a reference for request shapes and prompts, not
code to copy. "R N" refers to research.md, "DM" to data-model.md. Stages that spend money or Codex
quota are marked **(run)**; the maintainer has approved a spend of about $8 on the key (cap $15).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1 README chart, US2 reproducible run, US3 method page,
  US4 threshold and wording recommendation)

---

## Phase 1: Setup

**Purpose**: the `bench/` project, its checks and its CI, before any stage exists.

- [X] T001 Create `bench/pyproject.toml`: `[project]` name `bench`, version `0.0.0`, `requires-python = ">=3.14"`, dependencies `httpx` and `matplotlib`; `[dependency-groups] dev = ["ruff", "pyright", "pytest"]`; `[project.scripts] bench = "bench.cli:main"`; `[build-system]` `uv_build`; `[tool.ruff]` `line-length = 100`, `target-version = "py314"`, `[tool.ruff.lint] select = ["ALL"]` ignoring only `D` (docstrings; no-comment style), `CPY`, `COM812` and `ISC001` (formatter conflicts), with `[tool.ruff.lint.per-file-ignores] "tests/**" = ["S101", "PLR2004"]`; `[tool.pyright] typeCheckingMode = "strict"`, `pythonVersion = "3.14"`, `include = ["src", "tests"]`; `[tool.pytest.ini_options] testpaths = ["tests"]` (R10)
- [X] T002 Create `bench/.python-version` (`3.14`) and `bench/src/bench/__init__.py` (empty); run `uv lock` in `bench/` to create `bench/uv.lock` (R10)
- [X] T003 [P] Create `bench/check.ps1` modelled on the root `check.ps1`: `param([switch]$Fix, [string[]]$Stage = @('format','lint','types','test'), [string]$Filter)`, `$locked = @(if (-not $Fix) { '--locked' })`, stages `format` (`uv run @locked ruff format` or `--check` without `-Fix`), `lint` (`uv run @locked ruff check`, `--fix` with `-Fix`), `types` (`uv run @locked pyright`), `test` (`uv run @locked pytest`, `-k $Filter` when given); runs from its own folder (`Push-Location $PSScriptRoot`); fails on the first failing stage with a non-zero exit (contracts/cli.md)
- [X] T004 [P] Create `.github/workflows/bench.yml`: name `Bench`; `on.push.branches: [main]` and `on.pull_request`, both with `paths: ['bench/**', '.github/workflows/bench.yml']`; `permissions: {}` at the top and `contents: read` on the job; one job on `ubuntu-latest`: `actions/checkout` pinned by SHA as in `ci.yml` with `persist-credentials: false`, `astral-sh/setup-uv` pinned by full commit SHA with a `# vX.Y.Z` comment (look it up with `gh api repos/astral-sh/setup-uv/commits/<tag> --jq .sha`), then `./bench/check.ps1` with `shell: pwsh` (R11)
- [X] T005 [P] Append to `.gitignore` under `# Project`: `/bench/.cache/` and `/bench/.venv/`
- [X] T006 Create `bench/src/bench/cli.py`: `main()` with argparse subcommands `prepare`, `patterns`, `wordings`, `run`, `repeat`, `judge`, `score` and the options of contracts/cli.md (`--queries`, `--parallel`, `--continue-judging`), each dispatching to a stage function that for now raises `NotImplementedError`; exit status 0, 1 (error, message to stderr) or 3 (a `LimitReached` exception defined in `bench/src/bench/limits.py`, message says how to continue); `uv run bench --help` lists the stages

**Checkpoint**: `bench/check.ps1 -Fix` passes; `./check.ps1` is unaffected.

---

## Phase 2: Foundational

**Purpose**: storage, dataset, pool and labels, which every stage reads.

- [X] T007 Create `bench/src/bench/store.py`: the results root `bench/results/` and cache root `bench/.cache/` resolved from the package location; `write_json(path, value)` writes UTF-8, LF, two-space indent to a temporary file in the same folder and `os.replace`s it; `read_json(path)`; `exists(path)` as the unit-complete marker (R9, contracts/results.md)
- [X] T008 [P] Create `bench/tests/test_store.py`: a written file reads back equal; a failed write (exception during serialisation) leaves no file; LF line endings on every platform
- [X] T009 Create `bench/src/bench/dataset.py`: pinned source (`github/CodeSearchNet`, commit `106e827405c968597da938f6b373d30183918869`, `resources/annotationStore.csv`, R1); a thin `download()` into `.cache/`; pure functions: parse the CSV into Python-only rows, `Query` (DM: ids `q00`… by sorted text), expert rating per `(query, url)` as the mean over annotators, `relevant` when the mean is ≥ 2, the split (seed 5, 20 dev, the rest test) and 5 repeat queries drawn from test with the same seed; writes `results/split.json` (contracts/results.md)
- [X] T010 Create `bench/src/bench/pool.py`: pure parsing of a `GitHubUrl` (`owner/repo/blob/<sha>/<path>#L<a>[-L<b>]`, single-line anchors too) into a raw URL and a line range; pure cutting of a file to the range; a thin fetch with a file cache in `.cache/raw/`; the pool: distinct resolving URLs sorted, named `NNNN.py`, code written to `.cache/pool/NNNN.py`, `sha256` of the cut code; unresolving URLs listed as missing; writes `results/pool.json` (R2, DM Snippet)
- [X] T011 [P] Create `bench/tests/test_dataset.py` and `bench/tests/test_pool.py`: mean of duplicate ratings and the ≥ 2 rule; split and repeat draw are deterministic and disjoint; URL parsing (range, single line, a path with slashes), cutting (first and last line inclusive), names follow URL order, a missing URL is listed and not named
- [X] T012 Create `bench/src/bench/labels.py`: label of a pair from expert rating, else judge rating, else unknown; `relevant` as in DM; built from `split.json`, the expert ratings and any `results/judge/*.json` present
- [X] T013 [P] Create `bench/tests/test_labels.py`: expert wins over judge; judge fills gaps; unknown is neither relevant nor irrelevant
- [X] T014 Wire `prepare` in `bench/src/bench/cli.py` to dataset and pool; **(run, free)** `uv run bench prepare`: about 943 snippets, about 13 missing, 20 dev, 79 test, 5 repeat queries (quickstart check 1)

**Checkpoint**: `results/pool.json` and `results/split.json` exist; checks pass.

---

## Phase 3: User Story 2 - The maintainer runs the benchmark reproducibly (Priority: P1)

**Goal**: every stage from contracts/cli.md up to judging, resumable, producing the committed raw
results. Comes before US1 because the chart is drawn from these results.

**Independent Test**: run `run --queries` on two queries, interrupt a third, rerun: the first two are
not sent again; delete nothing and rerun `score` later without network (T041).

- [X] T015 [US2] Create `bench/src/bench/codex.py`: one function that runs `codex exec` (path from `shutil.which("codex")`, error if missing) with `-m gpt-6-astra -c model_reasoning_effort=low -s read-only --ephemeral --skip-git-repo-check --ignore-rules -C <.cache/codex-empty/> --output-schema <schema file> -o <output file> -`, the prompt on stdin, and returns the parsed JSON; a non-zero exit whose stderr reports a usage limit raises `LimitReached`; a helper runs a list of such calls with up to `--parallel` at once (thread pool) (R7)
- [X] T016 [US2] Create `bench/src/bench/grep.py`: the frozen filler-word list (`a an the to of from in on for and or is how with into by based another x`), keywords of a query (lowercase alphanumeric words minus fillers), and ripgrep runs over `.cache/pool/`: any keyword (`rg --files-with-matches --ignore-case -F -e w1 -e w2 …`), all keywords (per-word hit sets intersected), a given pattern (`-e <pattern>`); each returns sorted pool indexes and the wall seconds; `compiles(pattern)` runs rg on an empty folder and checks for a regex error (R5)
- [X] T017 [P] [US2] Create `bench/tests/test_grep.py`: keywords of sample queries; the three searches on a temporary folder of four small files (ripgrep, offline); an invalid pattern does not compile
- [X] T018 [US2] Create `bench/src/bench/patterns.py` and wire `patterns` in `cli.py`: batches of 20 queries, prompt: a coding agent searching a Python codebase for each query writes one case-insensitive ripgrep regex, as it would search (synonyms, identifier forms), seeing only the query; schema `{patterns: [{id, pattern}]}`; a pattern that does not compile is asked for once more alone, else the stage fails; writes `results/patterns.json` (R6)
- [X] T019 [US2] Create `bench/src/bench/jev.py`: questions JSON `{"match": {"type": "noul", "instructions": <question>}}`; runs `jevpipe map --read-files --model typesafe/jev-1.13 --concurrency 100 -q <json>` with the pool paths on stdin, the environment passed through; parses each stdout line (`answers.match.noul`, or `outcome` skipped/failed) into probabilities in pool order, the stderr summary into cost, exit status 3 into `LimitReached`; measures wall seconds around the process; `resolved_version()` makes one `POST https://openrouter.ai/api/v1/systemone` with a trivial noul question (shape in `specs/001-filter-foundation/api-spike.md`) and returns the response's `model` (R3)
- [X] T020 [US2] Create `bench/src/bench/deepseek.py`: async httpx client with 100 connections and a semaphore of 100; the request of R4 (system and user message, `temperature 0`, `max_tokens 5`, `reasoning.enabled false`, `logprobs true`, `top_logprobs 5`, `provider.require_parameters true`); three attempts per request; pure `answer_of(content)` (first word, lowercased, punctuation stripped) and `probability_of(top_logprobs)` (`p(yes) / (p(yes) + p(no))` over tokens equal to yes/no after trimming and lowercasing, `None` when neither is present); records provider and `usage.cost`; a 402 or a key-limit error raises `LimitReached`; returns answers and probabilities in pool order, failures, providers, cost, wall seconds, the responding model
- [X] T021 [P] [US2] Create `bench/tests/test_deepseek.py`: `answer_of` for `yes`, `No.`, `no\n\nthis function…`; `probability_of` for yes-first, no-first, both with leading spaces or capitals, neither present
- [X] T022 [US2] **(run, <$0.01)** Verify the DeepSeek request shape on 20 pool snippets of one query with a throwaway call through `deepseek.py`: every answer has a probability under `require_parameters`; record the outcome (and any change of approach) in research.md §4
- [X] T023 [US2] Create `bench/src/bench/wording.py` and wire `wordings` in `cli.py`: the four wordings of R8; for each model and wording, decide the dev queries' expert-rated pairs (jevpipe with the rated snippets' pool paths; DeepSeek on the same), F1 of relevant at 0.5 (Jev) or on the answer (DeepSeek); writes `results/wordings/jevpipe.json`, `results/wordings/deepseek.json` and `results/frozen.json` with the best wording per model (FR-016, contracts/results.md)
- [X] T024 [US2] Create `bench/src/bench/runs.py` and wire `run` and `repeat` in `cli.py`: for each query (or `--queries`) without `results/runs/<q>.json`: the three grep searches (pattern from `patterns.json`), then jevpipe and DeepSeek over the whole pool with the frozen question, jevpipe first on even query index and DeepSeek first on odd (FR-009); stored only when complete (DM Run states: at most 1% failed per model, no limit); one progress line per query and a summary with the spend; `repeat` does the same for the repeat queries into `results/repeat/<q>.json` with only the model entries; `LimitReached` ends the stage with status 3 without storing the query. The resolved Jev version is fetched once per stage and stored in every run (contracts/results.md)
- [X] T025 [US2] **(run, subscription)** `uv run bench patterns`; read the 99 patterns once for sanity (plausible, compile) and commit nothing yet
- [X] T026 [US2] **(run, ~$0.10)** `uv run bench wordings`; note the chosen wordings and their dev F1
- [X] T027 [US2] **(run, ~$0.15)** Trial `uv run bench run --queries q00,q01`: check quickstart check 2 (complete answers, probabilities, times and costs near the spike); stop the stage during a third query and rerun it to see the resume (US2 independent test)
- [X] T028 [US2] **(run, ~$7.3)** `uv run bench run` for all queries; if the key's limit stops it (status 3), ask the maintainer to raise it and rerun
- [X] T029 [US2] **(run, ~$0.4)** `uv run bench repeat`
- [X] T030 [US2] Create `bench/src/bench/judge.py` and wire `judge` in `cli.py`: selection by DM Judgment rules (validation, hit, grep-any-sample of 20, unflagged-sample of 10, each pair once, seed 5), shuffled across queries and reasons, opaque ids `j00001`…, written once to `results/judge/selection.json` and reused on rerun; batches of 40 in selection order; prompt: the CodeSearchNet scale and guidelines (from `scratch/bench/judge_proto.py`), then per item only id, query text and code; schema `{judgments: [{id, relevance 0..3}]}`; a batch with missing, repeated or unknown ids is sent again once, then its bad items are recorded as unjudged; each batch to `results/judge/<n>.json`; validation batches first, then the gate: the judge's precision, recall and F1 of relevant against the experts next to one expert against the others (first annotator of each multiply-rated pair against the mean of the rest), stop with status 3 below F1 0.67 unless `--continue-judging` (FR-010..FR-014)
- [X] T031 [P] [US2] Create `bench/tests/test_judge.py`: selection on a small fixture: every rated pair in validation, unrated hits of each model at 0.3 and of the two greps, the grep-any sample counts hits already selected, the unflagged sample excludes every flagged pair, no pair twice, determinism with the seed; batch split of 40; the agreement numbers on a hand-made fixture
- [X] T032 [US2] **(run, subscription)** `uv run bench judge` until the gate: report the judge's F1 next to the experts' 0.72 to the maintainer; below 0.67, stop and let the maintainer decide (FR-010)
- [X] T033 [US2] **(run, subscription)** `uv run bench judge` again (with `--continue-judging` only if the maintainer decided so) until every batch is judged; after a quota stop, rerun later

**Checkpoint**: `results/` holds pool, split, patterns, wordings, frozen, 99 runs, 5 repeats and all
judge batches; `bench/check.ps1` passes.

---

## Phase 4: User Story 1 - A developer sees what jevpipe gives their agent (Priority: P1) 🎯 MVP

**Goal**: the chart and the README section.

**Independent Test**: the chart alone answers which tool found most, which returned most false hits,
and jevpipe's cost per 1,000 and time per search (SC-001).

- [X] T034 [US1] Create `bench/src/bench/metrics.py`: pure functions over labels, runs and judgments: decision at a threshold per contender (DM Contender), thresholds chosen on dev (best F1 among 0.30…0.90 in 0.05 steps), per contender over the test queries found, false hits, precision, recall of known relevant, F1 (grep-any estimated from its sample, DM Result), time median and 90th percentile, cost per 1,000; the sweep 0.30…0.90 for both models; accuracy per confidence band; the estimate of relevant pairs no contender found; judge agreement; the share of decisions changed in the repeat runs; counts of missing snippets, failures, skips and answers without probability
- [X] T035 [P] [US1] Create `bench/tests/test_metrics.py`: a hand-computed fixture of two queries and a few snippets: found, false hits, precision, recall, F1 per contender; threshold choice on dev applied to test; the grep-any estimate; bands; the missed estimate; the repeat change share
- [X] T036 [US1] Load the `dataviz` skill, then create `bench/src/bench/chart.py`: the chart of contracts/docs.md and R13 (three tools, found and false hits on one axis, cost per 1,000 and median time as labels, a title stating the finding, footnote with dataset, date and versions, own light background, PNG at 2× for 800 px width) with fixed metadata so the same results give the same bytes
- [X] T037 [US1] Wire `score` in `cli.py`: reads only `results/`, writes `results/results.json`, `results/numbers.md` (all tables of the method page, including the mechanical greps, the sweep, the bands) and `results/chart.png`; prints the headline numbers; **(run, free)** `uv run bench score` and look at the chart
- [X] T038 [US1] Add the section of contracts/docs.md to `README.md` after the example: a heading answering the reader's question, the chart by absolute `raw.githubusercontent.com/fabianboth/jevpipe/main/bench/results/chart.png` URL with alt text stating the result, at most two sentences with dataset, date and model versions, an absolute link to `bench/README.md`; no table

**Checkpoint**: the README renders the chart on GitHub in light and dark mode (after merge; before,
check the file on the branch).

---

## Phase 5: User Story 3 - A sceptical reader checks the method (Priority: P2)

**Goal**: the method page.

**Independent Test**: every choice that could favour jevpipe is stated with its reason (US3).

- [X] T039 [US3] Write `bench/README.md` with sections 1-4, 6 and 7 of contracts/docs.md: what was measured and why; dataset, pool, label gaps and the judge with its validation next to the experts' agreement; contenders with exact settings, all four wordings and the chosen ones, thresholds and how they were chosen; the results from `results/numbers.md` (copied in, with the date); limitations; how to rerun with the stages, costs and resuming

---

## Phase 6: User Story 4 - Evidence for the defaults and the skill (Priority: P3)

**Goal**: the recommendation.

**Independent Test**: the page states jevpipe's precision and recall at 0.3, 0.5 and 0.7 on the test
queries and the wording chosen on dev, with a recommendation.

- [X] T040 [US4] Add section 5 of contracts/docs.md to `bench/README.md`: the recommended `filter` threshold for search with its precision and recall on the test queries, whether the default 0.5 should change, and the question wording that worked best; the change itself stays out of this milestone

---

## Phase 7: Polish

- [X] T041 Verify SC-003: with the network off (or `score` run in a fresh clone of the branch), `uv run bench score` finishes in under a minute and `git status` shows `results/` unchanged
- [X] T042 [P] Update `CLAUDE.md`: in Layout, `bench/` for the benchmark (its own uv project, run by hand, results committed); in Verification, `bench/check.ps1` (ruff, pyright strict, pytest, offline) and the path-filtered `.github/workflows/bench.yml`; keep the rest unchanged
- [X] T043 [P] Check the committed `bench/results/` for snippet code or secrets (none allowed, FR-005, FR-029) and its total size (about 1-2 MB expected)
- [X] T044 `bench/check.ps1 -Fix` and `./check.ps1 -Fix` pass; set the spec's Status to Complete; walk through quickstart.md once more against what was built

---

## Phase 8: Review follow-up

A review of the finished branch; the stored runs and ratings stay as they are, `score` regenerates
everything else.

- [X] T045 One rule for "a tool flagged this" (`decisions.py`) used by the judge's selection, the
  scoring and the wording trials; contender names in one place (`contenders.py`); run formats in
  `records.py`
- [X] T046 Split `judge.py` into `selection.py`, `agreement.py` and the judge stage; split `score.py`
  into `report.py`, `numbers.py` and the stage; pooling through `Counts` sums and exact band merges
- [X] T047 Fairness: DeepSeek retries like jevpipe (five attempts, transient errors only,
  `Retry-After`, no wait after the last attempt); `--retry-failed` covers both models; a retry's time
  counts with its cost
- [X] T048 Robustness: jevpipe's exit 2 without a summary is a fatal error; timeouts for jevpipe and
  Codex; a failing Codex task cancels the waiting ones; downloads fail loudly unless gone; atomic
  cache writes; `prepare` refuses to replace a pool that stored runs use; a failed license lookup is
  not stored as "no license"
- [X] T049 Honest numbers: values never measured are `null`; the any-keyword grep's found count uses
  the same estimate as its answer key; the bootstrap resamples within each language; the judge is
  also compared like for like with the experts; chart and card text computed from the data
- [X] T050 Tests for pooling, titles, the wording tie-break, the card, figure determinism, jevpipe
  against a stand-in binary and DeepSeek against a mock transport; the method page checked against
  every results file

## Dependencies & Execution Order

- **Setup (T001-T006)** → **Foundational (T007-T014)** → **US2 (T015-T033)** → **US1 (T034-T038)** →
  **US3 (T039)** → **US4 (T040)** → **Polish (T041-T044)**.
- US1 depends on US2's results (the chart is drawn from them), although both are P1; US3 and US4 need
  US1's `numbers.md`.
- Within US2: T015 before T018 and T030; T016 before T018 and T024; T019 and T020 before T022-T024;
  the run tasks in order T025 → T026 → T027 → T028 → T029 → T032 → T033 (patterns and wordings are
  frozen before any pool run; judging needs all runs).
- Judging (T032-T033) may span days if the Codex quota runs out; nothing else waits on it except US1.

## Parallel Opportunities

- Setup: T003, T004, T005 together after T001-T002.
- Foundational: T008, T011, T013 alongside their modules' successors.
- US2: T016 + T017, T019, T020 + T021 can be written in parallel after T015; T031 alongside T030.
- US1: T035 alongside T034.
- Polish: T042 and T043 together.

```text
# US2 implementation in parallel (different files):
T016 bench/src/bench/grep.py      T019 bench/src/bench/jev.py      T020 bench/src/bench/deepseek.py
T017 bench/tests/test_grep.py                                      T021 bench/tests/test_deepseek.py
```

## Implementation Strategy

1. **MVP path**: Setup → Foundational → US2 through the trial run (T027) → a first `score` on the two
   trial queries to see the chart shape early (T034-T037 can be written against the trial data).
2. Then the full run, repeat and judging (T028-T033), and `score` for real.
3. README section (US1), method page (US3), recommendation (US4), polish.
4. Commit per phase; the spend so far and the state of `results/` go into each commit message.

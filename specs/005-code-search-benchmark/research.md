# Research: Semantic Code Search Benchmark

Decisions for the plan, from the spike (`scratch/bench/`, 2026-09-27) and the sources named. The spike
code is a prototype; the benchmark is rebuilt in `bench/`.

## 1. Dataset source and pinning

- **Decision**: Download `resources/annotationStore.csv` from `github/CodeSearchNet` at commit
  `106e827405c968597da938f6b373d30183918869` (HEAD on 2026-09-27) through
  `raw.githubusercontent.com`. Fetch each snippet from the commit its `GitHubUrl` names, cut to the
  named line range.
- **Rationale**: The repository is archived; pinning the commit makes the input exact. Fetching from
  the rated commit is what the experts saw. The spike fetched 976 of 989 Python pairs (13 dead links).
- **Alternatives considered**: The CodeSearchNet corpus dump (large, hosted on S3, and the rated
  snippets must be matched back anyway); Hugging Face mirrors (not the published source).

## 2. Pool and neutral names

- **Decision**: The pool is every distinct resolving rated Python snippet (943 in the spike), sorted by
  source URL and named `NNNN.py` in that order. jevpipe and DeepSeek both see the file name
  and content.
- **Rationale**: `jevpipe --read-files` judges path and content, so the path must not hint at the
  query; the same name goes into the DeepSeek prompt so both see the same input. Sorting by URL makes
  the names reproducible.
- **Alternatives considered**: Real file paths (leak repository and function names, which grep would
  also exploit, but differently per tool); records as lines (snippets are multi-line).

## 3. Jev model version

- **Decision**: Run `jevpipe map --model typesafe/jev-1.13`. Record the resolved version with one
  direct call to the API at run start (the response names it, e.g. `typesafe/jev-1.13-20260917`),
  since jevpipe does not print it.
- **Rationale**: TypeSafe's model docs (docs.typesafe.ai/models.md): the only release is jev-1.13.0,
  `jev-latest` is an alias that "moves when a new release ships"; pinning keeps one model for the
  whole run. The 001 API spike shows the response's `model` field carries the dated version.
- **Alternatives considered**: `~typesafe/jev-latest` (could move mid-run).

## 4. DeepSeek request shape

- **Decision**: `deepseek/deepseek-v4.1-flash` through OpenRouter chat completions: system message
  "You judge one file at a time. Answer with exactly one word: yes or no.", user message
  `{question}\n\nFile: {name}\n\n{code}`, `temperature 0`, `max_tokens 5`,
  `reasoning: {enabled: false}`, `logprobs: true`, `top_logprobs: 5`, and
  `provider: {require_parameters: true}`. The probability of yes is
  `p(yes) / (p(yes) + p(no))` over the first token's top alternatives. Up to 100 requests in flight,
  three attempts per request. Record `provider` and `usage.cost` per request. (After the runs, the
  review aligned the retries with jevpipe's: five attempts, transient errors only, `Retry-After`;
  the probability comes from the first token that holds yes or no.)
- **Rationale**: The spike's shape worked (0 errors over 956 + 1,886 requests). Without
  `require_parameters`, 75 of 956 answers came from providers that dropped logprobs; OpenRouter's
  provider-routing docs say `require_parameters: true` routes only to providers that support every
  parameter in the request. The first implementation step verifies that it also enforces logprobs;
  if not, answers without a probability stay (spec edge case) and are counted.
- **Verified 2026-09-27** (T022): `require_parameters` alone is not enough. Novita lists `logprobs`
  and `top_logprobs` as supported but returned no logprobs in 25 of 25 answers (10 of the first 20
  snippets); the other providers returned them. The request therefore also sets
  `provider.ignore: ["Novita"]`, and an answer without a probability is asked again (up to three
  attempts, all paid attempts counted in the cost); only if every attempt lacks one does the answer
  stay without a probability. With this, 20 of 20 answers carried a probability, served by seven
  providers.
- **Alternatives considered**: Pinning one provider with `order` (steadier timing, but not what a user
  gets and a single point of failure); a JSON-schema answer (adds tokens and fails on some providers).

## 5. Grep baselines

- **Decision**: Run the real ripgrep over the pool folder, `rg --files-with-matches --ignore-case`,
  per query: any keyword (`-F -e w1 -e w2 …`), all keywords (the any-keyword hits intersected per
  word), and the agent-written pattern (`-e <pattern>`). Keywords are the query's lowercase words
  minus a fixed list of filler words, frozen in the code.
- **Rationale**: "An agent reaches for grep" means ripgrep; its regex dialect is the one the pattern is
  written for. It must be a program on the path: Claude Code's built-in `rg` is not one, and the PyPI
  `ripgrep` package has no Windows wheel. Here it is ripgrep 15.2.0 from scoop; CI installs Ubuntu's
  package. The version is stored with every run. The search passes `--no-ignore`, because the pool
  lives in the gitignored cache.
- **Alternatives considered**: Python `re` (different dialect from what the agent writes).

## 6. Agent-written patterns

- **Decision**: Codex (`gpt-6-astra`) writes one ripgrep pattern per query, from the query alone, in
  batches of 20 queries with an output schema `{patterns: [{id, pattern}]}`; the patterns are
  committed before any contender runs and validated by running `rg` on an empty folder (an invalid
  pattern is asked for again once).
- **Rationale**: The same agent that could search this way writes it; batching saves quota and each
  pattern sees only its query.
- **Alternatives considered**: Writing patterns by hand (the author knows the results), Claude writing
  them (this session has seen the data).

## 7. Judge

- **Decision**: `codex exec -m gpt-6-astra -c model_reasoning_effort=low -s read-only --ephemeral
  --skip-git-repo-check --ignore-rules -C <empty folder> --output-schema <schema> -o <file> -`, the
  prompt on stdin: the CodeSearchNet rating scale and guidelines, then items as `id`, query and code.
  Output schema `{judgments: [{id, relevance: 0..3}]}`. Batches of 40 items, items shuffled across
  queries and sources with a fixed seed, ids opaque (`j00001`). Up to 4 calls in parallel. Resolve
  the `codex` shim with `shutil.which` (Windows).
- **Rationale**: The spike's trial rated 5 items in 8.8 s, correctly. The output schema makes the
  answer machine-checked; read-only sandbox and an empty folder keep it from reading the dataset.
  About 6,000 items (validation 976, model and pattern hits ~2,000, grep samples ~2,000, unflagged
  sample ~1,000) are about 150 calls.
- **Alternatives considered**: One item per call (150× the calls and quota); OpenRouter models as
  judge (spends the key's budget; the user's subscription is otherwise unused).

## 8. Protocol numbers

- **Decision**: Seed 5 for everything random (split, samples, batch order). 20 dev queries. Four
  question wordings (below). Thresholds for the chart: per model, the best F1 on the dev queries'
  pool results, among 0.3 to 0.9 in steps of 0.05. Repeat run: 5 test queries drawn with the seed.
- **Wordings**:
  1. `Does this code do what a developer searching for "{query}" is looking for?` (spike)
  2. `Is this code a good result for the code search "{query}"?`
  3. `Would a developer searching for "{query}" want to use this function?`
  4. `Does this function implement "{query}"?`
- **Rationale**: Four wordings cover the plain forms an agent would write; more would start fitting
  the dev queries. The wording stage costs well under $0.10.

## 9. Storage

- **Decision**: JSON files under `bench/results/`, one per finished unit, written atomically (temp
  file, then rename): `runs/<query>.json` per query with the decisions as arrays in pool order,
  `judge/<batch>.json` per batch. Download cache and snippets in `bench/.cache/` (gitignored).
- **Rationale**: One file per unit is the resume marker (a unit exists or it does not); arrays in pool
  order keep all runs at about 1 MB instead of ~12 MB of per-pair records.
- **Alternatives considered**: SQLite (binary, poor diffs); one JSONL per stage (a crash can leave a
  half-written line and resuming needs parsing).

## 10. Python toolchain

- **Decision**: Python 3.14 (pinned in `.python-version`, installed 3.14.2), uv project with the
  `uv_build` backend and a `bench` console script; dependencies `httpx` (async HTTP) and `matplotlib`
  (chart); dev group `ruff`, `pyright`, `pytest`. Pyright `typeCheckingMode = "strict"`; ruff with a
  broad rule set (`select = ["ALL"]` minus the rules that conflict with the formatter or the house
  style). Own `bench/check.ps1`: ruff format (or `-Fix`), ruff check, pyright, pytest, with `--locked`
  outside `-Fix`, mirroring the repository's `check.ps1`.
- **Rationale**: The user settled uv, ruff, pyright strict and pytest. Pyright has the highest
  typing-spec conformance (97.8%) and is the engine of VS Code's Pylance; ty is still beta (0.0.x);
  Pyrefly 1.0 (May 2026) is newer and less proven (pydevtools and danilchenko.dev comparisons, 2026).
  argparse suffices for a handful of stages.
- **Alternatives considered**: Typer or Click (a dependency for five subcommands); mypy (58%
  conformance).

## 11. CI

- **Decision**: A separate workflow, `.github/workflows/bench.yml`, runs `bench/check.ps1` (offline,
  no key) on `ubuntu-latest` for pushes to `main` and pull requests that change `bench/**` or the
  workflow itself (`on.<event>.paths`). It is not part of `ci-success`, and `ci.yml` is unchanged. No
  paid stage ever runs in CI.
- **Rationale**: Constitution II: committed code passes automated checks. The benchmark is not shipped
  code, so a failing check shows on the pull request without blocking the product's required gate
  (the user's call). GitHub's own path filter needs no third-party action. The job needs only uv and
  PowerShell, not Rust, and takes about a minute.
- **Alternatives considered**: A path-filtered job inside `ci.yml` that `ci-success` waits for (blocks
  merges, but needs a filter action and changes the product's gate); running the checks by hand only
  (committed code could rot unnoticed).

## 12. Packaging

- **Decision**: No change to `Cargo.toml` or `pyproject.toml` at the root.
- **Rationale**: The release builds only binary wheels (`bindings = "bin"`), which carry the binary and
  nothing from `bench/`; no sdist is built and the crate is `publish = false`. `bench/pyproject.toml`
  is its own project; the root `pyproject.toml` has no uv workspace, so they do not interact.

## 13. Chart

- **Decision**: One PNG at 2× pixel density, drawn with matplotlib on its own light background with
  rounded margins, committed at `bench/results/chart.png` and referenced from the README by absolute
  `raw.githubusercontent.com` URL. Per tool (agent-written grep, jevpipe, DeepSeek): two horizontal
  bars, relevant found and false hits, on one shared axis; the cost per 1,000 records and median time
  per search as the tool's label. Colours and type follow the dataviz skill, loaded when the chart is
  drawn.
- **Rationale**: A picture with its own background reads in GitHub's light and dark themes and on PyPI,
  whose README sanitiser may drop `<picture>` sources. PNG avoids SVG quirks on
  `raw.githubusercontent.com`. Showing only the agent-written grep keeps the axis readable.
- **Alternatives considered**: `<picture>` with light and dark variants (not reliable on PyPI); SVG.

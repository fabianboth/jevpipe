# Contract: the `bench` command

Run from `bench/` as `uv run bench <stage> [options]`. Stages run in this order; each reads what the
previous ones stored and resumes on its own. Paid stages need `OPENROUTER_API_KEY` in the
environment: jevpipe takes it from there, and the DeepSeek calls and the one Jev version call use it
directly; it is never written anywhere. Stages that use the agent need a logged-in `codex`.

| Stage | Does | Paid | Writes |
|---|---|---|---|
| `prepare` | download the ratings at the pinned commit, fetch and cut the snippets, build the pool and the dev/test split | no | `.cache/`, `results/pool.json`, `results/split.json` |
| `patterns` | ask the agent for one ripgrep pattern per query; check each compiles | subscription | `results/patterns.json` |
| `wordings` | try the four wordings for both models on the dev queries' rated pairs; freeze the best per model | ~$0.10 | `results/wordings/<model>.json`, `results/frozen.json` |
| `run` | per query: grep baselines, then jevpipe and DeepSeek over the whole pool in alternating order | ~$7.4 | `results/runs/<query>.json` |
| `repeat` | second run of jevpipe and DeepSeek for the 5 repeat queries | ~$0.4 | `results/repeat/<query>.json` |
| `judge` | select the pairs to judge (validation first), judge them in batches; stop after validation if the judge's F1 is below 0.67 | subscription | `results/judge/selection.json`, `results/judge/<batch>.json` |
| `score` | compute every metric for every judged language, pool the languages whose judge passed, draw the figures, write the numbers | no, offline | `results/**/results.json`, `results/**/numbers.md`, `results/languages.json`, `results/*.png` |
| `licenses` | look up each source repository's license on GitHub (`gh api`) | no, GitHub | `results/licenses.json` |
| `export` | write the dataset: queries, corpus (with code and license), qrels and the card | no, offline | `.cache/export/` |

## Options

| Option | Stages | Meaning |
|---|---|---|
| `--queries q03,q17` | `run`, `repeat` | only these queries (for a trial) |
| `--parallel N` | `judge`, `patterns` | Codex calls at once, default 4 |
| `--continue-judging` | `judge` | go on after a validation F1 below 0.67 (the maintainer's decision, FR-010) |
| `--language python\|java\|javascript\|php\|ruby\|go` | `prepare`, `patterns`, `run`, `judge` | the CodeSearchNet language; default `python`. Other languages keep Python's search ids, are all test searches and use Python's frozen wordings and thresholds; their results live in `results/<language>/` |
| `--retry-failed` | `run` | ask both models again about the records a stored run left unanswered (with `--queries`, only those), and record the retry |

## Output and exit status

Each stage prints one line per unit (query or batch) and a closing summary line with what it spent;
`score` prints the headline numbers.

| Status | Meaning |
|---|---|
| 0 | stage complete |
| 1 | an error stopped the stage (the message says which unit and why) |
| 3 | stopped by a limit: the key's spend limit, a used-up Codex quota, or the judge gate; finished units are kept, rerunning continues |

## The check script

`bench/check.ps1 [-Fix] [-Stage format,lint,types,test]`: ruff format (verify, or fix with `-Fix`),
ruff check, pyright strict, pytest; `uv sync --locked` outside `-Fix`. Offline, no key, no Rust.

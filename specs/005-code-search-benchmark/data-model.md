# Data Model: Semantic Code Search Benchmark

The entities are frozen dataclasses in `bench/src/bench/`; their stored form is in
[contracts/results.md](contracts/results.md).

## Query

| Field | Type | Rule |
|---|---|---|
| `id` | `str` | `q00` … `q98`, in the order of the query texts sorted |
| `text` | `str` | the Challenge's query, unchanged |
| `split` | `dev` \| `test` | 20 dev queries drawn with seed 5, the rest test |

## Snippet

| Field | Type | Rule |
|---|---|---|
| `name` | `str` | `NNNN.py`, index in the pool sorted by `url` |
| `url` | `str` | the Challenge's `GitHubUrl` (commit and line range) |
| `sha256` | `str` | of the cut code, to detect a changed download |

Distinct by `url`. A URL whose file does not resolve is not a snippet; it is listed as missing.

## Pair and labels

A pair is `(query, snippet)`. Its label comes from the first of:

1. **Expert**: the mean of the experts' ratings for the pair, if any rated it.
2. **Judge**: the judge's rating, if the pair was judged and has no expert rating.
3. **Unknown**: neither.

`relevant(pair)` is true when the label is 2 or more; unknown is neither relevant nor irrelevant.

## Contender and decision

| Contender | Decision per pair | Threshold |
|---|---|---|
| `jevpipe` | `probability: float \| None` (the noul), `None` when skipped or failed | flagged when `probability >= t` |
| `deepseek` | `answer: bool \| None`, `probability: float \| None`, `None` answer after 3 failed attempts | flagged when `probability >= t`; where there is no probability, when `answer` |
| `grep-any` | flagged or not | none |
| `grep-all` | flagged or not | none |
| `grep-agent` | flagged or not | none |

For the models, `t` is chosen on the dev queries (best F1 among 0.30 … 0.90 in steps of 0.05) and
applied unchanged to the test queries. A failed or skipped decision is never flagged.

## Run

One pass of one model over the pool for one query.

| Field | Type | Rule |
|---|---|---|
| `query` | `str` | query id |
| `model` | `str` | requested model id |
| `resolved_model` | `str` | the version that answered (Jev: from the start-of-run call; DeepSeek: from the responses) |
| `started` | ISO 8601 | |
| `wall_seconds` | `float` | from first request to last answer, the whole pool |
| `cost` | `float` | US dollars as OpenRouter reports |
| `decisions` | array in pool order | as above |
| `failed`, `skipped` | `int` | |
| `providers` | `{provider: count}` | DeepSeek only |
| `first` | `bool` | whether this model ran first for the query (alternates) |

**States**: absent → complete. A run is stored only when complete: jevpipe exited 0 or 2 with at most
1% failed records and not stopped by a limit (exit 3); DeepSeek with at most 1% failed requests.
Otherwise nothing is stored and the query is run again on resume.

## Wording trial

Per model and wording: the decisions on the dev queries' expert-rated pairs and their F1 at 0.5 (Jev)
or at the answer (DeepSeek). The best wording per model is frozen.

## Pattern

Per query: the ripgrep pattern the agent wrote, and whether it compiled on the first attempt.

## Judgment

| Field | Type | Rule |
|---|---|---|
| `id` | `str` | opaque `j00001` …; the judge sees only this, the query text and the code |
| `query`, `snippet` | ids | not sent to the judge |
| `reason` | `validation` \| `hit` \| `grep-any-sample` \| `unflagged-sample` | why it was judged, never sent |
| `relevance` | `0..3` | the judge's rating |
| `batch` | `int` | |

**Selection** (all for dev and test queries):

- `validation`: every pair with an expert rating.
- `hit`: every unrated pair flagged by `jevpipe` at 0.3, by `deepseek` (answer yes or probability 0.3
  or more), by `grep-all` or by `grep-agent`.
- `grep-any-sample`: per query, 20 unrated `grep-any` hits drawn with the seed (all, if fewer);
  pairs already selected as `hit` count toward the 20.
- `unflagged-sample`: per query, 10 unrated pairs flagged by no contender, drawn with the seed.

A pair is judged once, whatever the reasons; batches mix queries and reasons.

## Result

Computed over the test queries per contender:

| Metric | Rule |
|---|---|
| `found` | flagged and relevant; for `grep-any`, its relevant rated hits plus its unrated hits × the relevant share of its sample |
| `false_hits` | flagged and labelled not relevant; for `grep-any`, estimated the same way with the not-relevant share |
| `precision` | `found / (found + false_hits)` |
| `recall` | `found / known relevant`, where known relevant counts every relevant pair of the test queries, from any source |
| `f1` | harmonic mean |
| `seconds_median`, `seconds_p90` | of the per-query wall time (grep: ripgrep's wall time) |
| `cost_per_1000` | total cost / records × 1000 (grep: 0) |

Also: precision and recall per threshold 0.30 … 0.90 for both models; accuracy per confidence band
(`max(p, 1-p)` in tenths); the estimated relevant pairs no contender found (unflagged-sample relevant
share × unflagged unrated pairs); the judge's agreement with the experts; the share of decisions that
changed in the repeat runs; the counts of missing snippets, failures, skips and answers without a
probability.

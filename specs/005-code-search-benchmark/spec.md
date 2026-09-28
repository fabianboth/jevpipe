# Feature Specification: Semantic Code Search Benchmark

**Feature Branch**: `005-code-search-benchmark`
**Created**: 2026-09-27
**Status**: Complete
**Input**: User description: "005 Benchmark: jevpipe for semantic code search, measured on the CodeSearchNet Challenge. Goal: show coding-agent users what jevpipe gives them compared with grep and a cheap general LLM, as one chart for the README (plus a short method page), no tables in the README. Setup settled in the spike (scratch/bench): Python subset of the CodeSearchNet Challenge (99 queries, ~956 expert-labelled query-snippet pairs, relevance 0-3, relevant = mean >= 2); every query is searched over the whole pool of ~943 snippets (realistic repo search); contenders: jevpipe (Jev, pinned version), DeepSeek V4.1 Flash via OpenRouter (yes/no answer with logprob probabilities, same question text, 100 requests in flight like jevpipe), grep baselines (any keyword, all keywords, and an rg pattern written per query by a coding agent). Unlabelled hits are judged blind by GPT-6 Astra through the Codex CLI (user's ChatGPT subscription, not OpenRouter), validated first on the expert-labelled pairs against human agreement (F1 ~0.72); a random sample of items no tool flagged is also judged to estimate misses. Question wording is tuned on 20 dev queries (each model gets its best of the same variants), frozen, and results are reported on the remaining 79 queries; thresholds chosen on dev. Store per-pair probabilities, time, cost, errors, provider, model versions; resumable per query with a budget stop; OpenRouter spend for the full run about $7.5 within a $15 key cap. Rerun 5 queries to measure run-to-run variation. Metrics: relevant found, false hits, precision/recall, wall time per query over the pool, cost per 1,000 records. Lives in bench/ (own Python project with uv, not part of the Rust crate, not run by CI, excluded from the crate package); datasets downloaded to a gitignored cache, results and chart committed. Known limitations stated: possible training-data contamination, Python only, function-sized snippets, one task. Spike findings: Jev ~$0.022/1k and 8.7s per 943-file query vs DeepSeek ~$0.058/1k and ~21s; on annotated pairs Jev F1 0.65 at 0.5 / 0.71 at 0.3, DeepSeek 0.68, human ceiling 0.72."

## Context

jevpipe is released (0.1.0), but its README only claims what it gives a coding agent. A user deciding
whether their agent should reach for `jevpipe filter` instead of grep, or instead of a script that asks
a general LLM about every file, has no numbers to go on. This milestone measures one job, semantic code
search ("find the code that does X"), and puts the answer into the README as one chart.

The audience is the developer of a coding-agent workflow asking "what does this give me?", not the
research community. The comparison is therefore between the tools an agent can reach for, on the
things the agent pays for: relevant code found, false hits that land in its context, time and money.
It is a sound, reproducible measurement, stated with its limits, not a paper.

A spike (2026-09-27, `scratch/bench/`, about $0.80 spent) settled the setup:

- The Python part of the CodeSearchNet Challenge has 99 queries and 989 query-snippet pairs rated by
  experts (0 irrelevant to 3 exact match); 13 snippets no longer resolve. One expert agrees with the
  others at F1 0.72, which is the ceiling any tool can be scored at against these labels.
- Scored only on the rated pairs, every tool lands near that ceiling (jevpipe F1 0.65 at threshold 0.5
  and 0.71 at 0.3, DeepSeek V4.1 Flash 0.68, GPT-6 Luna 0.65, Claude Haiku 4.5 0.64), and grep looks
  good because the rated candidates were pre-selected by search systems that match words. That setting
  cannot show the difference that matters.
- Searching each query over the whole pool of about 943 snippets, as an agent searches a repository,
  shows it: for "all permutations of a list", grep on any keyword returns 326 snippets for 3 relevant
  ones, while jevpipe and DeepSeek find the 3 with no false hit. Over the same 943 files with 100
  requests in flight, jevpipe took 8.7 s and $0.020 per query, DeepSeek 20-23 s and $0.055.
- The pool has label gaps: snippets rated only for another query are unrated for this one, and some of
  them are correct finds (AES encryption code for "aes encryption"). Counting them as false hits would
  punish the tools that understand code, so unrated hits are judged. A trial with GPT-6 Astra through
  the Codex CLI rated such hits correctly (3 for the AES code, 0 for unrelated code).
- Haiku 4.5 costs 14 times as much as jevpipe per record with no better answers, and GPT-6 Luna is
  close to DeepSeek; DeepSeek V4.1 Flash, the cheapest general model with the best spike score, is the
  one LLM contender.

The results also answer two open questions from 004: which `filter` threshold suits search (the spike
hints 0.3 over the current default 0.5), and how to phrase a search question for the skill.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A developer sees what jevpipe gives their agent (Priority: P1)

A developer reads the jevpipe README. Near the top, one chart shows, for the same code search over the
same pool of snippets, how many relevant snippets each tool found and how many false hits it returned,
for grep, jevpipe and a cheap general LLM, with the cost per 1,000 records and the time per search
next to each tool. One or two sentences say what it shows, the date and the model versions, and link
to the method page. From the chart alone they can tell whether and when to have their agent use
jevpipe instead of grep or an LLM script.

**Why this priority**: It is the purpose of the milestone; everything else exists to make this chart
true and trustworthy.

**Independent Test**: Show the README chart to someone who has not read the method page and ask which
tool finds most relevant code, which returns most false hits, and which is cheapest and fastest; each
answer can be read off the chart.

**Acceptance Scenarios**:

1. **Given** a finished benchmark, **When** a reader opens the README on GitHub or PyPI, **Then** they see one chart (no table) comparing the agent-written grep, jevpipe and DeepSeek V4.1 Flash on relevant snippets found and false hits, each tool labelled with its cost per 1,000 records and time per search.
2. **Given** the chart, **When** a reader wants to know how it was made, **Then** a link next to it leads to the method page.
3. **Given** GitHub in dark or light mode, **When** the README renders, **Then** the chart is readable in both.
4. **Given** the chart's numbers, **When** someone traces any of them, **Then** it comes from the committed results of one dated run with named model versions.

---

### User Story 2 - The maintainer runs the benchmark reproducibly (Priority: P1)

The maintainer runs the benchmark from the repository in stages: prepare the data, tune the question on
the dev queries, validate the judge, run the contenders over the pool, judge the unrated hits, compute
the results and draw the chart. A stage interrupted by a failure, the key's spend limit or a used-up quota
resumes where it stopped without paying again for finished work. The spend limit on the API key stops a run that would cost too much; a query it cuts short is not counted as finished. Recomputing the results and the chart
from stored raw answers makes no paid call.

**Why this priority**: The chart is only as good as the run behind it; a run that cannot be resumed or
repeated would have to be paid for again, and the numbers must be regenerable when the model changes.

**Independent Test**: Run the contender stage on two queries, stop it during a third, start it again:
the first two are not sent again and the third completes. Delete the chart and recompute it: no paid
call is made and the chart is identical.

**Acceptance Scenarios**:

1. **Given** a fresh clone with the API key set, **When** the maintainer runs the stages in order, **Then** each produces its stored output and the last produces the chart and the numbers for the README and method page.
2. **Given** a stage stopped part-way, **When** it is run again, **Then** it continues with the first unfinished query or batch.
3. **Given** the API key's spend limit is reached during a query, **When** the stage stops, **Then** that query is not stored as finished, and after the limit is raised the stage reruns it and continues.
4. **Given** stored raw answers, **When** the results stage runs, **Then** it needs no network access and no key.
5. **Given** the repository's normal checks (format, lint, tests, deny), **When** they run locally or in CI, **Then** the benchmark is not part of them and needs none of its tools.
6. **Given** the benchmark's own checks, **When** they run without network or key, **Then** they verify formatting, linting, strict types and the metrics, and make no paid call.

---

### User Story 3 - A sceptical reader checks the method (Priority: P2)

A reader who doubts the chart, for example on Hacker News, opens the method page. It states the
dataset and pool, the contenders and their exact settings, the question wording and how it was chosen,
the dev/test split, how unrated hits were judged and how well the judge agreed with the experts, how
misses were estimated, the thresholds and why, run-to-run variation, the full precision and recall
numbers, and the known limitations. It says how to rerun the benchmark.

**Why this priority**: The chart persuades only if its method holds up to the first critical reader;
but the chart has value before the page is polished.

**Independent Test**: Give the method page to a reviewer and ask them to find an unstated choice that
could have favoured jevpipe; every choice they name is stated with its reason.

**Acceptance Scenarios**:

1. **Given** the method page, **When** a reader looks for the judge's quality, **Then** it shows the judge's agreement with the experts next to the experts' agreement with each other.
2. **Given** the method page, **When** a reader looks for how thresholds and wording were chosen, **Then** it shows they were fixed on the dev queries before the test queries were scored.
3. **Given** the method page, **When** a reader looks for limitations, **Then** it names possible training-data contamination, Python only, function-sized snippets, one task, recall measured against known relevant snippets, and the judge being a model.

---

### User Story 4 - The maintainer gets evidence for the defaults and the skill (Priority: P3)

From the same results, the maintainer learns which `filter` threshold gives the best balance for code
search, how much it matters, and which question wording worked best, so the default and the skill's
advice can be revised on evidence.

**Why this priority**: It turns the measurement into product decisions, but the decisions themselves
are taken afterwards with the user.

**Independent Test**: The method page states, for jevpipe, precision and recall at the thresholds
0.3, 0.5 and 0.7 on the test queries and the wording chosen on dev, with a one-paragraph recommendation.

**Acceptance Scenarios**:

1. **Given** the results, **When** the maintainer reads the recommendation, **Then** it names a threshold for search and the evidence for it, and whether the current default should change.

---

### Edge Cases

- A snippet's source no longer resolves: it is left out of the pool and every metric, and the count is reported.
- jevpipe skips or fails a record (for example a file it cannot read): the record counts as not flagged, and skips and failures are reported per contender.
- A DeepSeek request fails after its retries: it counts as not flagged and is reported; if more than 1% of a query's requests fail, the query is rerun.
- DeepSeek's answer carries no token probabilities (some providers do not return them): its yes/no answer still counts; the threshold sweep uses only answers with a probability, and the count without one is reported.
- DeepSeek answers with more than one word or with punctuation: the answer is read from its first word.
- The judge omits an item, repeats one or returns an invalid rating: the batch is sent again; an item that fails twice is reported as unjudged and left out of the metrics.
- The ChatGPT subscription's quota runs out during judging: judging stops and resumes later with the next unjudged batch.
- The jevpipe model version changes during a run: the run pins one version, so every query uses the same model.
- The same snippet is rated for several queries: it appears once in the pool; its ratings apply per query.
- Several experts rated the same pair: their mean rating is used, and relevant means a mean of 2 or more.
- The judge rates a pair the experts also rated (in validation): the expert rating wins in all metrics; the judge's rating is used only to measure agreement.

## Requirements *(mandatory)*

### Functional Requirements

**Data**

- **FR-001**: The benchmark MUST use the Python queries and ratings of the CodeSearchNet Challenge, downloaded from their published source at a pinned version, and the snippets from the exact commits the ratings name.
- **FR-002**: The pool MUST contain every distinct rated Python snippet that still resolves, each once; every query MUST be searched over the whole pool.
- **FR-003**: A pair MUST count as relevant when its mean expert rating is 2 or more; where no expert rated it, the judge's rating decides (FR-012).
- **FR-004**: Snippets MUST be presented to every contender under neutral names that carry no hint of the query or the rating.
- **FR-005**: Downloaded data and snippets MUST stay out of the repository (a local cache); committed results MUST identify snippets by their source reference, not contain their code.

**Contenders**

- **FR-006**: jevpipe MUST run as the released binary through `jevpipe map` with one yes/no question per query, reading the snippets as files, with a pinned Jev model version and 100 requests in flight; the probability of every answer MUST be kept.
- **FR-007**: DeepSeek V4.1 Flash MUST get the same question text for the same snippet (name and content), answer yes or no in one word, at temperature 0, with reasoning off and 100 requests in flight; its yes/no answer and, where available, the probability of yes from its token probabilities MUST be kept, with the serving provider.
- **FR-008**: Three grep baselines MUST run over the same pool, case-insensitive: any keyword of the query and all keywords of the query (the query's words without filler words, chosen mechanically), and one search pattern per query written by a coding agent from the query alone, as an agent would search (synonyms, identifier forms), before any result is seen, then frozen and committed. The agent-written pattern is the grep in the chart; the two mechanical baselines are reported on the method page.
- **FR-009**: For each query, the order in which jevpipe and DeepSeek run MUST alternate, so neither systematically runs first.

**Judging**

- **FR-010**: Before the judge's ratings are used, it MUST rate all expert-rated pairs, blind to the expert ratings; its agreement with the experts (precision, recall and F1 of "relevant") MUST be reported next to the experts' agreement with each other, and the judge MUST reach an F1 of at least 0.67 (within 0.05 of the experts' 0.72) or the maintainer decides before judging continues.
- **FR-011**: The judge MUST rate on the experts' scale and guidelines, see only the query and the snippet, never which contender flagged it, and get items of different queries and contenders mixed in each batch.
- **FR-012**: The judge MUST rate every unrated pair flagged by jevpipe at probability 0.3 or more, by DeepSeek (answer yes or probability 0.3 or more), by the all-keywords grep or by the agent-written pattern; and a random sample of 20 unrated any-keyword grep hits per query, whose rate stands for all of that baseline's unrated hits.
- **FR-013**: The judge MUST rate a random sample of 10 pairs per query that no contender flagged, to estimate the relevant snippets every tool missed; the estimate MUST be reported, and recall MUST be stated as recall of the known relevant snippets.
- **FR-014**: The judge MUST be GPT-6 Astra through the user's Codex subscription, with no tools and no file access beyond the items it is given; it is a model of neither contender's vendor.

**Protocol**

- **FR-015**: 20 queries MUST be chosen at random with a fixed, recorded seed as dev queries; the other 79 are test queries, and every reported result MUST come from the test queries.
- **FR-016**: Between three and five question wordings MUST be tried on the dev queries over their expert-rated pairs; each model MUST get the wording with its best F1 there, and the wordings MUST then be frozen and committed before the pool run.
- **FR-017**: The thresholds shown in the chart MUST be chosen on the dev queries' pool results (the best F1 per model) and then applied unchanged to the test queries.
- **FR-018**: Five test queries MUST be run a second time for jevpipe and DeepSeek, and the share of snippets whose decision changed MUST be reported per model.

**Measurement**

- **FR-019**: For each contender and each test query, the benchmark MUST record relevant snippets found, false hits, precision and recall; and for the two models the wall time of the whole search over the pool, the cost reported by OpenRouter, failed and skipped records.
- **FR-020**: Results MUST be reported as totals over the test queries (relevant found, false hits, precision, recall, F1), as the median and 90th percentile of the time per search, and as the cost per 1,000 records.
- **FR-021**: Precision and recall of jevpipe and DeepSeek MUST also be reported across thresholds from 0.3 to 0.9, including 0.3, 0.5 and 0.7, and their accuracy per confidence band; below 0.3 hits are not judged, so no lower threshold is reported.

**Outputs**

- **FR-022**: The chart MUST show, for the agent-written grep, jevpipe and DeepSeek V4.1 Flash, relevant snippets found and false hits over the test queries, with the cost per 1,000 records and median time per search as labels; it MUST read at README width in GitHub's light and dark themes.
- **FR-023**: The README MUST carry the chart, at most two sentences about it with the date and the model versions, and a link to the method page; no table.
- **FR-024**: The method page MUST state everything User Story 3 lists, the recommendation of User Story 4, and how to rerun the benchmark.
- **FR-025**: The raw answers, the judge's ratings, the frozen wordings and patterns, the dev/test split and the results MUST be committed, so the numbers and the chart can be regenerated without paid calls.

**Operation**

- **FR-026**: Every paid or quota-limited stage MUST save its work per query or per batch and resume without repeating finished work.
- **FR-027**: A query cut short by the API key's spend limit or by errors MUST NOT be stored as finished; the stage reruns it when resumed.
- **FR-028**: The benchmark MUST live in its own folder with its own tooling, outside the jevpipe crate, its packages and its checks; running it needs only the folder's own tool setup and the released jevpipe binary.
- **FR-030**: The benchmark's code MUST pass its own offline checks (formatting, linting, strict type checking, and tests of the metrics and pooling rules on small fixtures), which need no network, no key and no Rust toolchain; none of them runs a paid stage.
- **FR-029**: The API key MUST come from the environment or the keychain as for jevpipe itself; no key or token MUST be written to a file.

### Key Entities

- **Query**: one natural-language search from the Challenge; belongs to the dev or test set.
- **Snippet**: one function-sized piece of Python code from a named commit, with a neutral name in the pool.
- **Pair**: a query and a snippet; has at most one expert rating (mean of the experts) and at most one judge rating, and is relevant or not.
- **Contender**: a tool that flags pairs: jevpipe, DeepSeek V4.1 Flash, or one of three grep baselines; the two models carry a model version, a question wording and a threshold.
- **Decision**: a contender's answer for a pair: flagged or not, with the probability where the contender gives one.
- **Run**: one pass of a contender over the pool for one query, with its wall time, cost, failures, provider and date.
- **Result**: the metrics per contender over the test queries, from which the chart and the method page are made.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A reader of the README can answer, from the chart alone, which of the three tools found the most relevant snippets, which returned the most false hits, and what jevpipe costs per 1,000 records and per search in time.
- **SC-002**: The full run (tuning, pool run over 99 queries, second run of 5 queries) spends at most $10 of OpenRouter credit; judging spends none.
- **SC-003**: Recomputing all numbers and the chart from the committed results takes under one minute with no network access.
- **SC-004**: Restarting any stopped stage repeats no finished query or batch.
- **SC-005**: The judge's agreement with the experts is reported and reaches an F1 of at least 0.67.
- **SC-006**: Every number in the README and on the method page can be traced to the committed results of one dated run with its model versions.
- **SC-007**: The recommendation for the `filter` threshold states the precision and recall it gives on the test queries.

## Assumptions

- The CodeSearchNet Challenge ratings remain available from their published source, and the snippets' commits stay reachable on GitHub; the spike found 13 of 989 missing.
- The pooled setting stands in for searching a repository of about a thousand functions. A real repository is about one topic while the pool mixes 99, which can help grep (keyword hits cluster) or hurt it (common words appear everywhere); the method page states the difference without claiming a direction.
- Unrated pairs no contender flagged and the sample does not cover are treated as irrelevant; FR-013's sample estimates how often that is wrong.
- The same threshold for "relevant" (mean rating 2 or more) applies to expert and judge ratings.
- Costs are the costs OpenRouter reports for the run; prices on the date of the run are stated.
- Times are measured from one machine on one network, both models from the same place and in alternating order; they show the difference between the tools, not absolute latency elsewhere.
- The user's ChatGPT subscription has enough quota for about 6,000 judged pairs; judging can be spread over several days.
- The spike's code in `scratch/bench/` is a prototype; the benchmark is rebuilt cleanly.
- The terminal demo (GIF) for the README is separate work, not part of this milestone.

## Out of Scope

- Other tasks and datasets (log triage, commit or issue classification, paper screening) and other languages than Python.
- More LLM contenders (Claude Haiku 4.5, GPT-6 Luna, frontier models) beyond the spike's figures; an agent reading the files itself as a contender.
- Decomposed questions (several sub-questions per query combined), and embedding-based search.
- Changing the `filter` default threshold or the skill; this milestone recommends, the change follows.
- Evals of the agent skill (whether agents reach for jevpipe when they should).
- Running the benchmark in CI or on a schedule.

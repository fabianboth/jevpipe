# Contract: README section, chart and method page

## README

A short section after the example, titled for the reader's question (for example "Does it beat
grep?"):

- the chart, as an image with an absolute `https://raw.githubusercontent.com/fabianboth/jevpipe/main/bench/results/chart.png` URL and alt text that states the result in words;
- at most two sentences: what was searched (99 → 79 test queries of the CodeSearchNet Challenge over
  about 940 Python functions), the date and the model versions;
- a link to the method page (absolute GitHub URL, as all README links).

No table. The existing "Cost and speed" table stays as it is.

## Chart

- Three tools, top to bottom: grep (the agent-written pattern), jevpipe, DeepSeek V4.1 Flash.
- Per tool two bars on one shared axis: relevant snippets found (of the known relevant, stated in the
  axis or title) and false hits, over the 79 test queries.
- Next to each tool: cost per 1,000 records and median time per search, e.g. `$0.02 / 1k · 9 s`.
- A title that states the finding, a one-line footnote with dataset, date and model versions.
- Own light background; legible at 800 px wide in GitHub's light and dark themes and on PyPI.

## Method page (`bench/README.md`)

1. What was measured and why (the agent's question: grep, jevpipe or an LLM script).
2. Dataset, pool and the label gaps; the judge and its validation next to the experts' agreement.
3. Contenders with exact settings, wordings (all four, the chosen ones) and thresholds, and how they
   were chosen on the dev queries.
4. Results: the numbers from `numbers.md`, including the mechanical grep baselines, thresholds 0.3 to
   0.9, confidence bands, the estimate of what every tool missed, run-to-run variation, failures.
5. Recommendation: the `filter` threshold for search and the question wording, with the evidence.
6. Limitations: possible training-data contamination, Python only, function-sized snippets, one task,
   a pool of 99 topics rather than one repository, recall of the known relevant, a model as judge,
   one machine and network for the times.
7. How to rerun: the stages, what each costs, how to resume.

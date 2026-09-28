# Contract: README section, chart and method page

## README

A short section after the example, titled for the reader's question (for example "Does it beat
grep?"):

- the chart, as an image with an absolute `https://raw.githubusercontent.com/fabianboth/jevpipe/main/bench/results/chart.png` URL and alt text that states the result in words;
- at most two sentences: what was searched (the test searches of the pooled languages, linked to the
  published dataset on Hugging Face), the date and the model versions;
- a link to the method page (absolute GitHub URL, as all README links), worded as an invitation
  that names what is there beyond the chart.

No table. The existing "Cost and speed" table stays as it is.

## Chart

- Three tools, top to bottom: grep (the agent-written pattern), jevpipe, DeepSeek V4.1 Flash.
- Per tool two bars on one shared axis: relevant snippets found (of the known relevant, stated in the
  axis or title) and false hits, over the 79 test queries.
- Next to each tool: cost per 1,000 records and median time per search, e.g. `$0.02 / 1k · 9 s`.
- A title that states the finding, a one-line footnote with dataset, date and model versions.
- Own light background; legible at 800 px wide in GitHub's light and dark themes and on PyPI.

## Method page (`bench/README.md`)

Written like a short paper: the finding first, one figure per finding, the exact settings folded
away. Before the appendix it fits in about two screens.

1. Title and a four-sentence abstract with the headline numbers; Figure 1 (the README chart).
2. The question: grep, jevpipe or an LLM script, and why each is a plausible choice.
3. What is being tested: the CodeSearchNet Challenge in plain words and one search worked through
   end to end (the query, a relevant function with a link to its source, what each tool flagged).
4. How it was measured: a Mermaid flow (dataset, pool, dev/test, tools, labels, metrics) and a few
   bullets, including the judge's validation next to the experts' agreement.
5. Findings, each a heading that states it with a figure and a caption: totals per language and
   pooled (table, `languages.png`), then over the pooled languages' test searches precision against
   recall across thresholds (`tradeoff.png`), accuracy by confidence (`calibration.png`) and time
   per 1,000 functions with cost and run-to-run changes (`times.png`); question wordings on the
   Python dev queries (`wordings.png`). Titles that state a result are computed from the data.
6. Recommendation: the `filter` threshold and the question wording.
7. Limitations: contamination, thin expert ratings outside Python, the languages left out of the
   pool, function-sized snippets, one task, a pool of topics rather than one project, recall of the
   known relevant, a model as judge, one machine for times.
8. Appendix in collapsed sections: exact contender settings and wordings, labels and the judge
   and the estimate of what every tool missed, all numbers including the mechanical greps, how to
   run it again.

Figures are drawn by `score` in the chart's style (own light background, the validated palette:
jevpipe blue, DeepSeek aqua with direct labels, grep neutral gray), deterministic like the chart.

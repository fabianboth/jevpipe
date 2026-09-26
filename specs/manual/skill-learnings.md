# Notes for the agent skill

A handful of runs over this repository while building `filter` (2026-09-26). Observations, not
conclusions: the skill's own eval set has to confirm or drop them.

## Observed

- Broad questions also match documents *about* the topic: "Does this file talk to an HTTP API?" gave
  the API specs 0.95–0.99, the same as `src/service.rs`.
- At the default 0.5, weak matches came through (tests, docs at 0.5–0.7); at 0.9 only the intended
  files remained, in the two questions tried.
- For a question with good keywords, grep was faster (0.08 s vs 2.5 s), free and about as accurate.

## Open questions

- Which questions does jevpipe answer clearly better than grep? Candidates: concepts without a
  keyword, such as "does this function retry on failure".
- Which phrasings separate well, and is a higher default threshold justified?

## External guidance

TypeSafe's noul docs: use 0.5 when a false yes and a false no cost the same, raise the threshold when
acting on a false yes is expensive, and consider a middle band for review (their example: 0.2 / 0.8).

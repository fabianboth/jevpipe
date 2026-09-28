# Quickstart: running the benchmark

Needs: uv, PowerShell 7 (for the check script), ripgrep as a program on the path (`scoop install
ripgrep`, `brew install ripgrep` or `apt install ripgrep`; Claude Code's built-in `rg` is not one),
`jevpipe` 0.1.0 on the path, a logged-in `codex` with access to `gpt-6-astra`, and
`OPENROUTER_API_KEY` in the environment with about $6 of spend left on the key.

```sh
cd bench
./check.ps1                          # offline: format, lint, types, tests

uv run bench prepare                 # dataset, snippets, pool, split (free)
uv run bench patterns                # agent-written grep patterns (Codex)
uv run bench wordings                # pick a wording per model on dev (~$0.04)
uv run bench run --queries q00,q01   # trial on two queries (~$0.10), check the output
uv run bench run                     # all queries (~$4.5, ~75 min); resumable
uv run bench repeat                  # 5 queries again (~$0.25)
uv run bench judge                   # validation first, then hits and samples (Codex, ~15 min)
uv run bench score                   # offline: results.json, numbers.md, chart.png
```

## Checks along the way

1. After `prepare`: 943 snippets carrying 956 rated pairs, 31 URLs that no longer resolve; 20 dev
   and 79 test queries.
2. After the trial run: both models answered every snippet of both queries; DeepSeek answers carry a
   probability (`require_parameters` and Novita ignored, research §4); Jev about 6 s and $0.019
   per query, DeepSeek about 15-35 s and $0.027.
3. After validation in `judge`: the judge's F1 against the experts is printed; below 0.67 the stage
   stops with exit status 3 and the decision goes to the maintainer.
4. After `score`: `chart.png` shows the three tools with found and false hits and their labels;
   rerunning `score` without network gives the same files.

## When a stage stops

Run the same command again: finished queries and batches are skipped. If the key's spend limit
stopped `run`, raise the limit first; the cut-short query is run again from its start.

---
name: jevpipe
description: Runs many small judgments over a stream of items with the jevpipe CLI, a Unix pipe for typed decisions. `jevpipe filter "question"` works like a semantic grep and keeps the lines or files answered yes; `jevpipe map` asks several yes/no, pick-one or score questions per record and prints JSON lines for jq. Use this skill whenever a task means applying the same judgment to dozens or thousands of items that no exact pattern captures, even if the user never mentions jevpipe - finding files by what the code does rather than what it is called, triaging log lines, test failures, issues or diff hunks, labelling, bucketing or scoring a list, or deciding each step of a scripted loop. Not for exact text search (use grep or rg), counting, arithmetic or writing text.
license: Apache-2.0
compatibility: Requires the jevpipe CLI (0.1.0) on PATH and network access to OpenRouter; jq recommended for map.
metadata:
  jevpipe-version: "0.1.0"
---

# jevpipe

jevpipe turns a judgment into a pipe stage. Records stream in (plain lines, JSON lines, or file
paths), each one goes to Jev, TypeSafe's small decision model, through OpenRouter, and a calibrated
decision comes back: the probability of yes, the chosen option, or a score. You write the loop and act
on the result; jevpipe only decides. That keeps a thousand small judgments out of your context: you
read the handful of records that matter, not every record you had to look at.

This is written for jevpipe 0.1.0. When a flag is rejected or something behaves differently,
`jevpipe <command> --help` is the source of truth; it also lists exit statuses and examples.

## When to use it, and when not

Reach for it when:

- The same question applies to many items: "which of these 400 files handle retries?", "which log
  lines are real errors rather than noise?", "which failing tests look flaky?"
- The concept has no reliable keyword. "Does this function retry on failure?" finds a loop with
  backoff whether or not the word "retry" appears.
- A scripted loop needs a decision at every step (keep or skip, which bucket, how urgent) that you
  would otherwise make by reading each item yourself.

Use something else when:

- An exact string or a good keyword finds it: `rg` is faster (a tenth of a second against seconds),
  free and about as accurate. Try the keyword first when one exists.
- The task is counting, summing, sorting or parsing structure: `wc`, `jq`, `awk` or a short script.
- The task is producing text (summaries, fixes, messages): jevpipe only returns decisions.
- There are only a few items: read them. Every jevpipe run costs money and a few seconds.

The two combine well: narrow with `rg`, `find` or `git ls-files` first, then let jevpipe judge what
is left.

## Before the first run

Check that it is installed: `jevpipe --version`. If the command is not found, stop and tell the user
that jevpipe needs installing, with the link https://github.com/fabianboth/jevpipe (its README has a
one-line install). Do not install or build it yourself: which package manager ends up on their PATH is
the user's decision.

Without an API key, `filter` and `map` exit with status 2 and an error that names
`OPENROUTER_API_KEY`. Then ask the user to do one of these themselves, in their own terminal:

- run `jevpipe auth set-key`, which stores the key in the system keychain and asks for it with hidden
  input, or
- set `OPENROUTER_API_KEY` in their environment.

Leave the key entirely to the user: do not ask for it, look for it in files or the environment,
print it, put it on a command line, or run `jevpipe auth set-key` yourself. The key spends the user's
money, and anything that passes through your context can end up in transcripts, logs or commits.
When only the user ever touches it, there is nothing to leak.

## filter or map

`filter` asks one yes/no question and prints the records answered yes, unchanged and in input order.
Use it like grep:

```sh
git ls-files | jevpipe filter "Does this file retry failed network requests?" --read-files --max-cost 0.20
jevpipe filter "Is this line an error worth a closer look?" app.log --max-cost 0.50 | head -20
```

`map` asks several questions about every record, or questions whose answer is not yes/no. Each record
is sent once with all the questions, and one JSON line per record comes back:

```sh
jevpipe map --max-cost 0.50 -q '{
  "relevant": {"type": "noul", "instructions": "Is this failure worth a closer look?"},
  "kind": {"type": "choice", "instructions": "What kind of failure is this?",
           "criteria": {"flaky": "timing, network or infrastructure", "real": "a deterministic bug"}},
  "severity": {"type": "score", "instructions": "How severe is this failure?",
               "criteria": ["cosmetic", "annoying", "blocking"]}
}' failures.log > answers.jsonl
```

The question types:

- `noul`: yes/no, answered with the probability of yes.
- `choice`: one of 1 to 255 named options; `criteria` maps each name to a description.
- `score`: one level of an ordered scale of 2 to 10 levels; `criteria` lists them from low to high.

For a longer question set, write it to a file and pass `-f questions.json` instead of `-q`.

What goes in:

- **File paths**, one per line, from `git ls-files`, `rg --files -g '*.py'` or `find`, with
  `--read-files` so that each file's path and content are judged. Directories, empty and binary files
  are skipped; large files are cut to fit, and map marks such an answered row `"truncated": true`.
- **Lines** from logs, command output or `git log --oneline`, or one JSON object per line. Each line
  is one record, so flatten a multi-line item into one line first (for example with `jq -c`).

## Phrasing questions

- Ask about the record itself, and be specific. "Does this file retry failed network requests?"
  works better than "Is this about networking?": a broad topic also matches documents *about* the
  topic, and in early runs specs, docs and tests scored as high as the code itself.
- Ask one thing per question. Split "Is it slow and flaky?" into two questions in one `map`.
- Say what the record is when that is not obvious: "This is a line from an nginx access log. Was this
  request made by a bot?"
- For a `choice`, make the options exclusive and describe each one; add an `other` option when not
  every record fits.
- Try the question on a sample first (`head -20 app.log | jevpipe ...`) and read the results before
  running it over everything. It costs almost nothing and catches questions that are too broad.

## Thresholds and the review band

`filter` keeps a record when the probability of yes is at least `--threshold` (default 0.5). In
`map`, a `noul` answer carries that probability, and you choose the cut.

Current guidance, from TypeSafe's documentation and a few early runs, to be revisited once jevpipe's
calibration has been measured:

- Use 0.5 when a false yes and a false no cost the same.
- Go higher (0.8 or 0.9) when acting on a false yes is expensive, for example when you will edit or
  delete what is kept. In early runs over a code base, 0.9 left only the intended files where 0.5 also
  let tests and docs through.
- Use a middle band for review: act on 0.8 and above, drop below 0.2, and look at the records in
  between yourself. That is where the model is unsure, and it is usually a small set. `filter` only
  splits in two, so use `map` with one `noul` question for this.
- Pin the model with `--model typesafe/jev-1.13` when runs must be comparable over time; the default
  follows the latest Jev.

## map output with jq

Each line is either `{"record": "...", "answers": {...}}` or
`{"record": "...", "outcome": "skipped" | "failed", "reason": "..."}`. Inside `answers`, each question
has its own object:

- noul: `{"type": "noul", "noul": 0.93}`
- choice: `{"type": "choice", "choice": "flaky", "probabilities": {...}, "confidence": 0.97}`
- score: `{"type": "score", "score": 2, "legend": {"0": "cosmetic", "1": "annoying", "2": "blocking"}, ...}`,
  where `score` is the level's position, 0 being the lowest

A few usages:

```sh
jq -r 'select(.answers.relevant.noul >= 0.8) | .record' answers.jsonl
jq -r 'select(.answers.kind.choice == "flaky") | .record' answers.jsonl
jq -c 'select(.outcome == "failed")' answers.jsonl
```

Save map's output to a file when you will query it more than once: a second run pays again.

## Limits, exit status and resuming

Set `--max-cost` (in US dollars) on every run over more than a handful of records. Every record is a
paid request, and inputs are easily larger than expected (a `find` that walks into `node_modules`).
Start with a small cap, read the cost in the summary, then size the real run. Add `--max-time 10m`
when a deadline matters.

Standard error ends with a one-line summary such as
`412 records, 17 kept, 3 skipped, 0 failed, $0.041, 9.8s`. Read it: it tells you what the run cost and
whether anything failed.

| Status | filter | map |
|---|---|---|
| 0 | at least one record was kept | no record failed |
| 1 | no record was kept | - |
| 2 | a record failed, or an error stopped the run | the same |
| 3 | a limit stopped the run early | the same |

Status 1 from `filter` is an answer (nothing matched), not a failure; do not retry it.

On status 3, standard error names where to continue, for example
`stopped: spend limit $0.50 reached ($0.50 spent); input from line 1234 on was not processed`. The
output so far is complete and in order. Tell the user the limit was reached and ask before spending
more. If they agree, continue with only the rest instead of rerunning everything, which would pay
again for records already answered:

```sh
tail -n +1234 files.txt | jevpipe filter "..." --read-files --max-cost 0.50 >> kept.txt
```

This works only when the input is a file, so for a long run save the input first
(`git ls-files > files.txt`). A stop because the key's limit or the account's credits are used up
reads the same way; only the user can add credit.

On status 2, `map` marks each failed record with `"outcome": "failed"` and a reason; rerun just those
records if the reason looks temporary (the service was unavailable).

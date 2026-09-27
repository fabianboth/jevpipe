---
name: jevpipe
description: Bulk System 1 decisions from the shell. The jevpipe CLI sends every line, file or record to a small decision model that works as a classifier you configure with plain-language instructions, and returns only typed answers (yes/no, one choice, a score) with their confidence. Use it whenever the same judgment applies to dozens or thousands of items, even if jevpipe is not named - finding files by what the code does, triaging CI failures or logs, labelling issues, sorting commits for release notes, reviewing a large diff, moderating a queue. It judges each item on its own and returns answers, not text.
license: Apache-2.0
compatibility: Requires the jevpipe CLI on PATH and network access to OpenRouter; jq recommended for map.
---

# jevpipe

jevpipe is a System 1 you can program from the shell. Pipe in lines, files or records, ask one
question, and a small, fast model answers it for each: yes or no, which category, what score, each
with its confidence. Hundreds of items take seconds and a fraction of a cent, and none of them pass
through your context. Hand it the reflexive judgments, and keep your own reasoning for what the
answers show.

## When to use it

Reach for it when you are about to go through many items one by one and judge each the same way:
opening dozens of files to find the ones that matter, reading a long log for the real errors,
labelling every issue or commit. It fits when:

- **each item can be judged from its own content**, without the others or the rest of the codebase;
  only its first 100,000 characters are read, so a longer file is judged on its beginning;
- **the judgment is quick**: a yes/no, a category or a score a reader would give at a glance, not a
  deep analysis;
- **there are enough items** that reading them yourself would cost real context or time: dozens and
  up.

## How to use it

Every run has the same shape:

```sh
<records> | jevpipe filter "<question>" | <act on the kept records>
<records> | jevpipe map -q '<questions>' | jq '<select and reshape>' | <act on the result>
```

Input is the same for both commands and works like `grep`: lines come from a pipe or from the files
you name. Each line is judged as it is; with `--read-files`, it is a path, and the file it names is
judged instead:

```sh
git log --format=%s | jevpipe filter "<question>"            # each commit message
jevpipe filter "<question>" app.log                         # each line of app.log
git ls-files | jevpipe filter "<question>" --read-files     # each file
```

Output: `filter` passes the matching lines on unchanged, `map` prints one JSON line per record.
From there, `head`, `sort`, `wc -l`, `xargs` or a loop acts on the result.

## filter: one yes/no question

Keeps the lines answered yes, unchanged and in order; the summary goes to standard error.

```sh
git log --format=%s 15.0.0..15.1.0 | jevpipe filter "Is this a new feature?"
```

```
ignore/types: add `ssa` type
printer: add Cursor hyperlink alias
jevpipe: 19 records, 2 kept, 0 skipped, 0 failed, $0.000225, 1.1s
```

## map: typed answers

Asks one or more named questions per record and prints one JSON line per record, in input order.

```sh
git log --format=%s 15.0.0..15.1.0 | jevpipe map -q '{
  "kind": {
    "type": "choice",
    "instructions": "What kind of change is this, for the release notes?",
    "criteria": {
      "feature": "a new capability for users",
      "fix": "a bug fix users would notice",
      "docs": "documentation only",
      "internal": "refactoring, tests, CI, dependencies or release chores"
    }
  }
}'
```

One line of its output:

```json
{
  "record": "printer: add Cursor hyperlink alias",
  "answers": {
    "kind": {
      "type": "choice",
      "choice": "feature",
      "probabilities": {"docs": 0.05, "feature": 0.87, "fix": 0.03, "internal": 0.05},
      "confidence": 0.82
    }
  }
}
```

- `noul` is yes/no: `{"type": "noul", "noul": 0.93}`, the probability of yes.
- `choice` picks one named option; `criteria` maps each name to a description.
- `score` rates on 2 to 10 levels you list from low to high, and answers with a position on that
  scale.
- A record without an answer (an empty or binary file, a failed request) has `"outcome"` and
  `"reason"` instead of `"answers"`.

Pipe it into jq to pick what you need:

```sh
... | jq -r 'select(.answers.kind.choice == "fix") | .record'
```

To look at the same answers in several ways, save them first (`> answers.jsonl`), since every run
is paid again:

```sh
jq -r 'select(.answers.kind.confidence < 0.5) | .record' answers.jsonl   # the unsure ones
jq -c 'select(.outcome == "failed")' answers.jsonl                       # records without an answer
```

## Writing questions

The answers follow the wording closely, so say exactly what you mean. A question about a topic also
matches files that only discuss it: ask for "source code that calls an HTTP API", not "files about
HTTP APIs".

Pick the type by the shape of the answer, as [TypeSafe's docs](https://docs.typesafe.ai/primitives.md)
recommend:

- **`noul`** for a clean yes/no. Ask one condition per question ("angry and asking for a refund?" is
  two), phrase it so that yes is the high value, and make the boundary clear: "any Python
  experience?" leaves no middle ground. For a subtle boundary, add `criteria` with `true` and
  `false` descriptions.
- **`choice`** for one of a known set with no order. Give the full list, describe each option so it
  stands apart from the others ("a bug fix users would notice", not just "fix"), and add an `other`
  option when the list may not cover every record.
- **`score`** for a position on a spectrum. Describe situations, not degrees ("broken, but a
  workaround exists", not "moderately severe"), keep to one dimension per question, and use only as
  many levels as you can describe distinctly.

## Reading the answers

TypeSafe's guidance ([confidence](https://docs.typesafe.ai/confidence.md)):

- A `noul` is the probability of yes. Use 0.5 (`filter`'s default `--threshold`) when both
  mistakes cost the same, go higher when acting on a false yes is expensive, and lower when missing
  a true yes is. Values in between can go to review instead: their example acts above 0.8 and
  rejects below 0.2.
- A `choice` or `score` comes with a `confidence`: act on high confidence, check medium, and
  don't act on low. Where the lines go depends on the stakes, and a high confidence alone does not
  prove an answer right.
- A `score` is the probability-weighted mean of the level numbers, from 0 to the top level, so it
  can fall between two levels: 1.43 on a three-level scale means between the second and third level,
  closer to the second.

## Cost

Every record is a paid request, and the cost follows its size: about $0.01 per 1,000 short lines and
$0.20 per 1,000 source files. If the user has set a spending cap (`jevpipe config get max-cost`), a
run stops sending requests once it reaches it.

Narrow the input first by what is certain, such as file types, directories or leaving out vendored and
generated files (`git ls-files '*.rs'`). That saves cost and keeps unrelated records out.

## Exit status

- 1 from `filter`: nothing matched. That is an answer, not an error.
- 2: a record failed, or an error stopped the run.
- 3: the spending cap or time limit stopped the run; standard error names the line to resume from.

## Setup and more

Installation and everything else: https://github.com/fabianboth/jevpipe.

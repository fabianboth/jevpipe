<div align="center">

<h1>jevpipe</h1>

<p><strong>Give your agent a System 1.</strong></p>

<p>Fast, cheap judgments over thousands of files, lines or records in one shell command,<br>so your agent decides at scale instead of reading everything itself.</p>

<p>
<a href="https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml/badge.svg"></a>
<a href="https://pypi.org/project/jevpipe/"><img alt="PyPI" src="https://img.shields.io/pypi/v/jevpipe"></a>
<a href="https://github.com/fabianboth/jevpipe#license"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue"></a>
</p>

<p>
<a href="https://github.com/fabianboth/jevpipe#quick-start">Quick start</a> ·
<a href="https://github.com/fabianboth/jevpipe#examples">Examples</a> ·
<a href="https://github.com/fabianboth/jevpipe#use-cases">Use cases</a> ·
<a href="https://github.com/fabianboth/jevpipe#measured">Measured</a> ·
<a href="https://github.com/fabianboth/jevpipe#cost-and-speed">Cost</a>
</p>

</div>

## Quick start

```sh
uv tool install jevpipe                             # the CLI
jevpipe auth set-key                                # your API key, kept in the system keychain
npx skills add fabianboth/jevpipe --skill jevpipe   # teaches your agent when and how to use it
```

You need an [OpenRouter API key](https://openrouter.ai/settings/keys), or a
[TypeSafe API key](https://console.typesafe.ai/keys) after `jevpipe config set provider typesafe`.
Where there is no keychain, as in containers, CI or headless Linux, set `OPENROUTER_API_KEY` or
`TYPESAFE_API_KEY` instead; it also takes precedence.
No uv yet? [Install it](https://docs.astral.sh/uv/getting-started/installation/) or use pipx.

<details>
<summary>Without uv, or with the GitHub CLI</summary>

```sh
pipx install jevpipe                                # instead of uv
gh skill install fabianboth/jevpipe jevpipe         # the skill, with the GitHub CLI
```

</details>

## Examples

**One yes/no question with `filter`.** Which of the 19 commits in ripgrep 15.1 are new features?
Ask each commit message:

```sh
# in a clone of https://github.com/BurntSushi/ripgrep
git log --format=%s 15.0.0..15.1.0 | jevpipe filter "Is this a new feature?"
```

```
# the commit messages that match the question
ignore/types: add `ssa` type
printer: add Cursor hyperlink alias

# the summary, on standard error
jevpipe: 19 records, 2 kept, 0 skipped, 0 failed, 5.7k tokens, $0.000225, 1.1s
```

**Typed answers with `map`**, next to a `filter`, on 1,000 messages from a bank's support inbox
([banking77](https://github.com/PolyAI-LDN/task-specific-datasets)): who wants to leave, and which
team should answer each message?

<img alt="jevpipe filter keeps the 13 of 1,000 bank support messages whose customer threatens to leave; jevpipe map then routes all 1,000 to the cards, transfers, account or fraud team, counted with jq, sort and uniq." src="https://raw.githubusercontent.com/fabianboth/jevpipe/main/demo/demo.gif" width="800">

## Use cases

Each item is judged on its own, so ask what the item itself can answer:

| Job | Question for every item | Answer |
|---|---|---|
| Search code by meaning | Does this file retry failed requests? | yes/no |
| Triage CI failures | Timeout, network error, failed assertion or crash? | choice |
| Review a large diff | Does this change touch authentication or permissions? | yes/no |
| Label an issue backlog | Bug report, feature request or question? | choice |
| Route a support inbox | Which team? How urgent? | choice, score |
| Moderate a comment queue | Fine, spam or abusive? | choice |
| Screen papers | How relevant is this abstract to my question? | score 1 to 5 |

## Measured

**Classification.** Jev against general LLMs on intent routing and prompt-injection detection,
from an [independent study](https://www.ayautomate.com/blog/jev-vs-llm-benchmark):

<img alt="Mean over three labelled tasks: Jev 1.13 reached 83.2% accuracy at $0.023 per 1,000 decisions and 0.33 s, GPT-5.4 nano 83.1% at $0.12, Claude Haiku 4.5 81.3% at $0.65, Gemini 3.5 Flash-Lite 80.9% at $0.16, and GPT-5.6 Terra 86.7% at $1.08." src="https://raw.githubusercontent.com/fabianboth/jevpipe/main/bench/results/classification.png" width="800">

**Code search.** jevpipe in an agent's hands: 470 searches from the
[extended CodeSearchNet Challenge](https://huggingface.co/datasets/Scoolar/codesearchnet-challenge-extended)
in five languages, each over every function of its language.

<img alt="Over 470 code searches in five languages, jevpipe found 1,272 relevant functions with 767 false hits, grep with an agent-written pattern 1,027 with 1,155, and DeepSeek V4.1 Flash 1,173 with 712." src="https://raw.githubusercontent.com/fabianboth/jevpipe/main/bench/results/chart.png" width="800">

**[Read the full benchmark](https://github.com/fabianboth/jevpipe/blob/main/bench/README.md)**:
every language, speed and cost, and the searches where jevpipe loses.

## Commands

| | Asks | Prints |
|---|---|---|
| `jevpipe filter "question"` | one yes/no question | the lines answered yes, like grep |
| `jevpipe map -q '{...}'` | several typed questions: yes/no, one choice, a score | one JSON line per record |

Both read their input like grep: lines through a pipe or from the files you name. With
`--read-files`, each line is a path, and the file it names is judged instead:

```sh
git log --format=%s | jevpipe filter "Is this a new feature?"                       # each commit message
jevpipe filter "Is this an error worth a closer look?" app.log                      # each line of app.log
git ls-files | jevpipe filter "Does this file retry failed requests?" --read-files  # each file
jevpipe map -f team.json inbox.txt | jq -r .answers.team.choice | sort | uniq -c    # each line of inbox.txt, counted by team
```

`jevpipe <command> --help` has the rest.

## Cost and speed

Every record is one request billed to your [OpenRouter](https://openrouter.ai) or
[TypeSafe](https://typesafe.ai) credit, and the cost follows the size of each record: about a cent per 1,000 commit messages, 4 to 18 cents per
1,000 source files. Asking several questions at once barely changes it. Up to 100 records run at
the same time, so hundreds take seconds.

`--max-cost 0.50` stops sending requests once the run has spent $0.50; it then exits with status 3
and names the line to resume from. On TypeSafe, which reports tokens rather than dollars, use
`--max-tokens 5M` instead.
To cap every run by default, run `jevpipe config set max-cost 0.50` once; `jevpipe config --help`
lists the other defaults you can set.

## License

Licensed under either of [Apache-2.0](https://github.com/fabianboth/jevpipe/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/fabianboth/jevpipe/blob/main/LICENSE-MIT), at your option. Unless you
explicitly state otherwise, any contribution you intentionally submit for inclusion in jevpipe, as
defined in the Apache-2.0 license, is dual licensed as above, without any additional terms or
conditions.

jevpipe is an independent
project, not affiliated with or endorsed by TypeSafe AI; the answers come from
[Jev](https://typesafe.ai), TypeSafe AI's calibrated decision model, through OpenRouter or
TypeSafe's own API. Building
from source needs Rust (rustup picks the pinned toolchain) and PowerShell 7 for `./check.ps1`.

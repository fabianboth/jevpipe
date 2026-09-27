<div align="center">

<h1>jevpipe</h1>

<p><strong>Give your coding agent a System 1.</strong></p>

<p>Fast, cheap judgments over thousands of files, lines or records in one shell command,<br>so your agent decides at scale instead of reading everything itself.</p>

<p>
<a href="https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml/badge.svg"></a>
<a href="https://pypi.org/project/jevpipe/"><img alt="PyPI" src="https://img.shields.io/pypi/v/jevpipe"></a>
<a href="https://github.com/fabianboth/jevpipe/blob/main/LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
</p>

<p>
<a href="https://github.com/fabianboth/jevpipe#quick-start">Quick start</a> ·
<a href="https://github.com/fabianboth/jevpipe#example">Example</a> ·
<a href="https://github.com/fabianboth/jevpipe#use-cases">Use cases</a> ·
<a href="https://github.com/fabianboth/jevpipe#cost-and-speed">Cost</a>
</p>

</div>

## Quick start

```sh
uv tool install jevpipe                             # the CLI
jevpipe auth set-key                                # your OpenRouter key, kept in the system keychain
npx skills add fabianboth/jevpipe --skill jevpipe   # teaches your agent when and how to use it
```

You need an [OpenRouter API key](https://openrouter.ai/settings/keys). Where there is no keychain,
as in containers, CI or headless Linux, set `OPENROUTER_API_KEY` instead; it also takes precedence.
No uv yet? [Install it](https://docs.astral.sh/uv/getting-started/installation/) or use pipx.

<details>
<summary>Without uv, or with the GitHub CLI</summary>

```sh
pipx install jevpipe                                # instead of uv
gh skill install fabianboth/jevpipe jevpipe         # the skill, with the GitHub CLI
```

</details>

## Example

Which of the 19 commits in ripgrep 15.1 are new features? Ask each commit message:

```sh
# in a clone of https://github.com/BurntSushi/ripgrep
git log --format=%s 15.0.0..15.1.0 | jevpipe filter "Is this a new feature?"
```

```
# the commit messages that match the question
ignore/types: add `ssa` type
printer: add Cursor hyperlink alias

# the summary, on standard error
jevpipe: 19 records, 2 kept, 0 skipped, 0 failed, $0.000225, 1.1s
```

<details>
<summary>Typed answers with <code>map</code>: sort every commit into feature, fix, docs or internal</summary>

```sh
# in a clone of https://github.com/BurntSushi/ripgrep
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

Every commit becomes one JSON line. One of the 19:

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

All 19 took 1.1 seconds and cost $0.0003. Pick from them with jq, for example the fixes:
`jq -r 'select(.answers.kind.choice == "fix") | .record'`. A low confidence marks an answer worth
a second look.

</details>

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
```

`jevpipe <command> --help` has the rest.

## Cost and speed

Every record is one request billed to your [OpenRouter](https://openrouter.ai) credit. The cost
follows the size of each record; asking several questions at once barely changes it. Up to 100
records run at the same time, so hundreds take seconds. Measured runs:

| Input | Records | Time | Cost | Per 1,000 records |
|---|---:|---:|---:|---:|
| Commit messages (ripgrep 15.0) | 136 | 2.0 s | $0.0016 | $0.012 |
| Small source files (jevpipe) | 31 | 1.4 s | $0.0013 | $0.043 |
| Source files (ripgrep) | 88 | 2.2 s | $0.016 | $0.18 |

`--max-cost 0.50` stops a run once it has spent $0.50; it then exits with status 3 and names the
line to resume from.
To cap every run by default, run `jevpipe config set max-cost 0.50` once; `jevpipe config --help`
lists the other defaults you can set.

## License

[Apache-2.0](https://github.com/fabianboth/jevpipe/blob/main/LICENSE). jevpipe is an independent
project, not affiliated with or endorsed by TypeSafe AI; the answers come from
[Jev](https://typesafe.ai), TypeSafe AI's calibrated decision model, through OpenRouter. Building
from source needs Rust (rustup picks the pinned toolchain) and PowerShell 7 for `./check.ps1`.

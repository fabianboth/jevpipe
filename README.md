# jevpipe

[![CI](https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml/badge.svg)](https://github.com/fabianboth/jevpipe/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/jevpipe)](https://pypi.org/project/jevpipe/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](https://github.com/fabianboth/jevpipe/blob/main/LICENSE)

A Unix pipe for typed decisions: stream records in, get calibrated decisions out.

```sh
git ls-files src | jevpipe filter "Does this file parse command line arguments?" --read-files
```

```
src/cli.rs
src/lib.rs
src/settings.rs
jevpipe: 31 records, 3 kept, 0 skipped, 0 failed, $0.001326, 1.4s
```

Like grep, but the match is a question. Each record is answered by
[Jev](https://typesafe.ai), TypeSafe AI's small decision model, through
[OpenRouter](https://openrouter.ai), and only the decisions come out: the kept lines for `filter`,
one JSON line of typed answers per record for `map`. Written for scripts and coding agents that need
many small judgments without reading every item themselves.

## Install

```sh
uv tool install jevpipe
```

Or `pipx install jevpipe`. No uv yet? Install it with
`curl -LsSf https://astral.sh/uv/install.sh | sh` (macOS, Linux) or
`powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"` (Windows).
If `jevpipe` is not found afterwards, run `uv tool update-shell` and open a new terminal. Upgrade with
`uv tool upgrade jevpipe`.

The package is the native binary for Linux, macOS and Windows (x64 and arm64); nothing runs through
Python.

## API key

jevpipe needs an [OpenRouter API key](https://openrouter.ai/settings/keys). Store it in the system
keychain:

```sh
jevpipe auth set-key
```

Or set `OPENROUTER_API_KEY`, which takes precedence and is the way to go on headless Linux.

## Examples

Keep the log lines worth a look:

```sh
jevpipe filter "Is this line an error worth a closer look?" app.log | head -20
```

Ask several typed questions per record with `map`, and select with [jq](https://jqlang.org/download/):

```sh
git log --format=%s -8 | jevpipe map -q '{
  "fix":  {"type": "noul", "instructions": "Does this commit message describe a bug fix?"},
  "area": {"type": "choice", "instructions": "Which part of the project does this commit change?",
           "criteria": {"cli": "commands and flags", "docs": "specs and documentation", "ci": "build and release"}}
}' | jq -r 'select(.answers.fix.noul >= 0.8) | .record'
```

Each line of `map`'s output looks like this:

```json
{"record":"Reject probabilities outside 0..=1 from the service","answers":{"fix":{"type":"noul","noul":0.89},"area":{"type":"choice","choice":"cli","probabilities":{"cli":0.87,"docs":0.12,"ci":0.01},"confidence":0.8}}}
```

A `noul` is a yes/no answer with its probability, a `choice` picks one of named options, a `score`
rates on an ordered scale. `jevpipe filter --help` and `jevpipe map --help` list every flag, the
question format and the exit statuses; `jevpipe config --help` shows how to change the defaults.

## Use it from a coding agent

```sh
npx skills add fabianboth/jevpipe
```

Or `gh skill install fabianboth/jevpipe jevpipe`. The skill teaches the agent when jevpipe beats
reading or grepping, how to phrase questions and pick thresholds, and to cap every run's cost. It
never handles your API key.

## Cost

Every record is one request billed to your OpenRouter credit. The run above cost a tenth of a cent,
but a large input adds up, so cap it:

```sh
git ls-files | jevpipe filter "Does this file retry failed requests?" --read-files --max-cost 0.50 --max-time 10m
```

A run stopped by a limit exits with status 3 and names the input line to resume from.

## Unofficial

jevpipe is an independent project, not affiliated with or endorsed by TypeSafe AI. Jev is TypeSafe
AI's model.

## License

[Apache-2.0](https://github.com/fabianboth/jevpipe/blob/main/LICENSE). Building from source needs
Rust (rustup picks the pinned toolchain) and PowerShell 7 for `./check.ps1`.

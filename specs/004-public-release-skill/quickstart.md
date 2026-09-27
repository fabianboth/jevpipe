# Quickstart: Release Runbook and Checks

Steps marked **(maintainer)** need the maintainer's accounts or a click in a browser. Everything else
can be run by an agent. The repository is already public, with secret scanning, push protection and a
read-only default token.

## 1. Before the tag (on `004-public-release-skill`)

1. `./check.ps1 -Fix` passes (includes the docs drift test).
2. Locally: `uvx maturin build --release` produces `jevpipe-0.1.0-py3-none-win_amd64.whl`; its
   METADATA shows the README as description, `Apache-2.0` and the repository URL;
   `uv tool install --no-index --find-links target/wheels jevpipe` in an isolated `UV_TOOL_DIR`
   gives a working `jevpipe --version`.
3. Notify hooks: the gitignored `.claude/settings.local.json` holds them, `.claude/scripts/notify.ps1`
   stays untracked (`.git/info/exclude`); a new session in this repository still notifies, and the
   repository's `.claude/settings.json` has no `hooks`.
4. Environments exist: `gh api repos/fabianboth/jevpipe/environments --jq '.environments[].name'`
   lists `pypi` and `testpypi`; `pypi` has the maintainer as required reviewer; both allow only
   version tags.
5. **(maintainer)** Pending publishers registered for project `jevpipe`:
   - pypi.org → Account → Publishing → GitHub: owner `fabianboth`, repository `jevpipe`, workflow
     `release.yml`, environment `pypi`.
   - test.pypi.org, the same with environment `testpypi`.

## 2. Merge and protect `main`

1. PR from `004-public-release-skill`, CI green (including `ci-success`), merge.
2. Create the ruleset on `main` (research R9: deletion, non_fast_forward, pull_request with 0
   approvals, required status check `ci-success`, no bypass actors) with
   `gh api -X POST repos/fabianboth/jevpipe/rulesets`.
3. Check: a direct `git push origin HEAD:main` of a scratch commit is refused.

## 3. Release v0.1.0

1. On `main` (version `0.1.0` in `Cargo.toml`): `git tag v0.1.0 && git push origin v0.1.0`.
2. `release.yml`: six wheels built (musl, the arm runners and Windows arm64 are the risks); six
   checks green; upload to TestPyPI; https://test.pypi.org/project/jevpipe/ shows the README with
   working links; the `pypi` job waits for review.
3. **(maintainer)** Approve the `pypi` deployment on the run's page (one click).
4. https://pypi.org/project/jevpipe/ shows 0.1.0 with attestations on each file; a GitHub Release
   `v0.1.0` with notes and the six wheels; `uv tool install jevpipe` on any machine installs 0.1.0.

On a failure before the approval: fix on a branch, merge, and **(maintainer)** move the tag
(`git tag -f v0.1.0 && git push -f origin v0.1.0`) and let it run again. Nothing reached PyPI.

## 4. Skill

1. In a temporary project: `npx skills add fabianboth/jevpipe` → only `SKILL.md` arrives (under
   50 KB); the agent lists the skill `jevpipe`. Also `gh skill install fabianboth/jevpipe jevpipe`.
2. **(maintainer, costs a few cents)** In a coding agent with the skill and a key, in this
   repository: "which files deal with retrying failed requests?" → the agent runs `jevpipe filter`
   with `--max-cost` and acts on the result; "find all calls of `retry_with_backoff`" → it uses a
   text search. Without a key: the agent asks the user to run `jevpipe auth set-key` or set
   `OPENROUTER_API_KEY` and never asks for the key.

## 5. README

A person who has not seen jevpipe follows the README from install to a first `filter` run on their
own files in under five minutes (SC-003), and can say what jevpipe does after the first screen
(SC-003a). The PyPI page renders the same README with working links.

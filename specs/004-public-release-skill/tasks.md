---
description: "Task list for the first public release and the agent skill"
---

# Tasks: First Public Release and the Agent Skill

**Input**: Design documents from `specs/004-public-release-skill/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/release.md](contracts/release.md),
[contracts/skill.md](contracts/skill.md), [contracts/readme.md](contracts/readme.md),
[quickstart.md](quickstart.md)

**Tests**: FR-019 requires one offline integration test (`tests/cli/docs.rs`, the real binary via
`assert_cmd`, no network). The release pipeline is verified by its own check job and the runbook
(quickstart.md), not by `cargo test`.

**Rules for every task**: follow `CLAUDE.md` (no comments, private by default, lint levels only in
`Cargo.toml`, `#![expect(clippy::unwrap_used)]` only at the test crate root, which `tests/cli/main.rs`
already has). No `src/` changes. Workflows: `permissions: {}` at the top, per-job minimum
permissions, `actions/checkout` with `persist-credentials: false`, third-party actions pinned by full
commit SHA with the version as a trailing `# vX.Y.Z` (look the SHA up with
`gh api repos/<owner>/<repo>/commits/<tag> --jq .sha`), `shell: bash` for steps that must run on
Windows too. Anything that runs in a browser or under the maintainer's accounts is marked
**(maintainer)**; prepare it and hand it over. `./check.ps1 -Fix` passes at the end of each phase.
"R N" refers to research.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1 install, US2 release, US3 skill, US4 README,
  US5 public repository)

---

## Phase 1: Setup

**Purpose**: the license and package metadata every later file refers to.

- [X] T001 [P] Create `LICENSE` with the full, unmodified Apache License 2.0 text from https://www.apache.org/licenses/LICENSE-2.0.txt (FR-021)
- [X] T002 [P] In `Cargo.toml` `[package]`, add `license = "Apache-2.0"` and `repository = "https://github.com/fabianboth/jevpipe"` after `description`; keep `publish = false` (FR-021, R3)

**Checkpoint**: `./check.ps1 -Fix` passes (cargo-deny ignores the private crate's own license).

---

## Phase 2: Foundational

**Purpose**: none. The five stories share only the Setup files; US2 builds on US1's
`pyproject.toml`, and the release itself (Phase 8) needs every story.

---

## Phase 3: User Story 1 - Install jevpipe with one command (Priority: P1) 🎯 MVP

**Goal**: a wheel that `uv tool install` turns into the bare native `jevpipe` command.

**Independent Test**: build the wheel locally and install it with uv in an isolated tool directory;
the installed `jevpipe` is byte-identical to the built binary and prints its version.

- [X] T003 [US1] Create `pyproject.toml` at the repository root (R3): `[build-system] requires = ["maturin>=1.15,<2"]`, `build-backend = "maturin"`; `[project]` with `name = "jevpipe"`, `dynamic = ["version"]`, `description` (the `Cargo.toml` description), `readme = "README.md"`, `license = "Apache-2.0"`, `license-files = ["LICENSE"]`, `requires-python = ">=3.8"`, `keywords` (`cli`, `llm`, `classification`, `coding-agents`, `unix-pipe`), `classifiers` (`Environment :: Console`, `Intended Audience :: Developers`, `Operating System :: OS Independent`, `Programming Language :: Rust`, `Topic :: Utilities`), `[project.urls]` `Repository` and `Issues`; `[tool.maturin] bindings = "bin"`, `strip = true`. No sdist is ever built or uploaded
- [X] T004 [US1] Verify locally (quickstart §1.2), all output under `scratch/`: `uvx maturin build --release --out scratch/wheels` builds `jevpipe-0.1.0-py3-none-win_amd64.whl` (it needs `README.md` to exist: create a one-line placeholder only if T013 is not done yet, and never commit it); unzip it and confirm `METADATA` has `License-Expression: Apache-2.0`, the repository URL and the README as description; install with `UV_TOOL_DIR=scratch/uvtools UV_TOOL_BIN_DIR=scratch/uvbin uv tool install --no-index --find-links scratch/wheels jevpipe`, confirm `scratch/uvbin/jevpipe.exe` has the same SHA-256 as the `jevpipe.exe` inside the wheel and prints `jevpipe 0.1.0`; then `uv tool uninstall jevpipe` with the same environment variables

**Checkpoint**: US1's packaging works on this machine; the other five platforms are proven by the
release run's `check` job.

---

## Phase 4: User Story 2 - Release a version by pushing a tag (Priority: P1)

**Goal**: `release.yml` builds, checks and publishes on a version tag, gated by one approval.

**Independent Test**: the v0.1.0 run in Phase 8 (quickstart §3). Before that: the workflow file is
valid (`gh workflow view release.yml` after pushing the branch lists it) and `actionlint`, if
available via `uvx`/prebuilt binary in `scratch/`, reports nothing.

- [X] T005 [US2] Create `.github/workflows/release.yml` (contracts/release.md, R4, R5), `name: Release`, `on: push: tags: ["v[0-9]+.[0-9]+.[0-9]+"]`, `permissions: {}`, jobs:
  - `build`: matrix of the six targets and build runners from data-model.md (`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` on `ubuntu-latest`, `x86_64-apple-darwin` on `macos-15-intel`, `aarch64-apple-darwin` on `macos-14`, `x86_64-pc-windows-msvc` on `windows-2022`, `aarch64-pc-windows-msvc` on `windows-11-arm`), `fail-fast: true`, `permissions: contents: read`; steps: checkout; a `shell: bash` step that fails unless `"v$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"` equals `$GITHUB_REF_NAME`; `PyO3/maturin-action` v1.51.0 with `maturin-version: v1.15.0`, `target: ${{ matrix.target }}`, `manylinux: musllinux_1_1` for the musl targets and `auto` otherwise, `args: --release --locked --out dist --compatibility pypi` plus `--compatibility 2_17` for the musl targets; `actions/upload-artifact` of `dist/*.whl` as `wheel-${{ matrix.target }}`
  - `check`: `needs: build`, matrix `os: [ubuntu-latest, ubuntu-24.04-arm, macos-15-intel, macos-latest, windows-latest, windows-11-arm]`, `permissions: contents: read`; steps: `actions/download-artifact` with `pattern: wheel-*`, `path: wheels`, `merge-multiple: true`; `astral-sh/setup-uv` (current release); a `shell: bash` step with `OPENROUTER_API_KEY` unset: `uv tool install --no-index --find-links wheels jevpipe`, then run `"$(uv tool dir --bin)/jevpipe"` so PATH does not matter: `--version` must print exactly `jevpipe ${GITHUB_REF_NAME#v}`; `echo x | jevpipe filter "q"` must exit 2 with standard error containing `OPENROUTER_API_KEY`
  - `testpypi`: `needs: check`, `runs-on: ubuntu-latest`, `environment: { name: testpypi, url: https://test.pypi.org/p/jevpipe }`, `permissions: id-token: write`; download the wheels into `dist/` (same pattern, merged); `pypa/gh-action-pypi-publish` v1.14.2 with `repository-url: https://test.pypi.org/legacy/` (review: no `skip-existing`)
  - `pypi`: `needs: testpypi`, `environment: { name: pypi, url: https://pypi.org/p/jevpipe }`, `permissions: id-token: write`; download the wheels into `dist/`; `pypa/gh-action-pypi-publish` v1.14.2 (attestations are on by default)
  - `github-release`: `needs: pypi`, `permissions: contents: write`; download the wheels into `dist/`; `gh release create "$GITHUB_REF_NAME" --repo "$GITHUB_REPOSITORY" --verify-tag --generate-notes dist/*.whl` with `GH_TOKEN: ${{ github.token }}`
- [X] T006 [US2] Create the GitHub environments (R4, data-model.md "Repository settings") with `gh api`: `testpypi` via `PUT repos/fabianboth/jevpipe/environments/testpypi` with `{"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}` and `POST .../environments/testpypi/deployment-branch-policies` `{"name":"v[0-9]*.[0-9]*.[0-9]*","type":"tag"}`; `pypi` the same plus `"reviewers":[{"type":"User","id":<id of fabianboth from gh api users/fabianboth --jq .id>}]` and `"prevent_self_review":false`. Verify with `gh api repos/fabianboth/jevpipe/environments --jq '.environments[] | {name, protection_rules}'` (quickstart §1.4)
- [X] T007 [US2] **(maintainer)** Hand over the pending-publisher registration (quickstart §1.5): on pypi.org → Account → Publishing and on test.pypi.org the same, a GitHub publisher with owner `fabianboth`, repository `jevpipe`, workflow `release.yml`, environment `pypi` (resp. `testpypi`), project name `jevpipe`. The release in Phase 8 waits for this
- [X] T008 [US2] Update `CLAUDE.md`: in the intro "Shape" paragraph mention that releases ship as binary wheels on PyPI (`uv tool install jevpipe`); add a short "Releases" section: a `vMAJOR.MINOR.PATCH` tag equal to the `Cargo.toml` version runs `.github/workflows/release.yml` (wheels via maturin, check on six platforms, TestPyPI, approval, PyPI, GitHub Release); `pyproject.toml` holds the PyPI metadata; `README.md` is also the PyPI description, so its links must be absolute

**Checkpoint**: `release.yml` exists and is valid; environments exist; the maintainer has the
publisher values.

---

## Phase 5: User Story 3 - A coding agent learns when and how to use jevpipe (Priority: P2)

**Goal**: `skills/jevpipe/SKILL.md` per contracts/skill.md, kept in step with the CLI by a test.

**Independent Test**: `cargo test docs` passes; the skill folder holds only `SKILL.md`, under 50 KB;
after the merge, `npx skills add fabianboth/jevpipe` installs only that folder (quickstart §4).

- [X] T009 [US3] Load the `skill-creator` skill (Skill tool, `anthropic-skills:skill-creator`) and follow its guidance for writing (not its eval loop, which is 005). Read `specs/manual/skill-learnings.md`, `specs/manual/idea-draft.md` and the real help of every command (`cargo run -q -- --help`, `filter --help`, `map --help`, `config --help`, `auth --help`) before writing
- [X] T010 [US3] Write `skills/jevpipe/SKILL.md` (contracts/skill.md, FR-011–FR-018): the frontmatter exactly as in the contract (`name: jevpipe`, a third-person, "a little bit pushy" `description` under 1024 characters naming the bulk tasks and what it is not for, `license: Apache-2.0`, `compatibility`, `metadata: jevpipe-version: "0.1.0"`, no `allowed-tools`); the eight body sections in the contract's order, imperative voice, examples, reasons instead of capitalised rules, ~150–220 lines; the missing-binary pointer is `https://github.com/fabianboth/jevpipe`; thresholds and phrasing marked as current guidance to be revisited after calibration; every example command uses only flags that exist in the help. No other file in `skills/jevpipe/`
- [X] T011 [US3] Create `tests/cli/docs.rs` and add `mod docs;` to `tests/cli/main.rs` (R10, FR-019): helper `help_text()` = the concatenated stdout of `jevpipe <cmd> --help` for `filter`, `map`, `config`, `auth` and each `config`/`auth` subcommand, through the existing `home::jevpipe()`/`home::stdout()`; helper that reads a repository file via `env!("CARGO_MANIFEST_DIR")`; helper that extracts every `--[a-z][a-z-]*` word from a text. Tests: `skill_flags_exist_in_the_help` (every flag in `skills/jevpipe/SKILL.md` appears in `help_text()`, failure message names the flag); `skill_name_matches_its_folder` (the frontmatter line `name: jevpipe` between the first two `---` lines equals the folder name `jevpipe`). Assert behaviour, not authored values: the expected name comes from the folder path

**Checkpoint**: `./check.ps1 -Fix` passes with the skill and its tests.

---

## Phase 6: User Story 4 - A newcomer understands jevpipe from the README (Priority: P2)

**Goal**: a landing-page README per contracts/readme.md that also renders correctly on PyPI.

**Independent Test**: `cargo test docs` passes; the README is under ~120 lines; after the release
the PyPI page renders it with working links (quickstart §3.4, §5).

- [X] T012 [US4] **(maintainer, costs a few cents)** Capture the real first-screen example (plan.md design 6): with the maintainer's key available (keychain or `OPENROUTER_API_KEY`), run in this repository `git ls-files | cargo run -q --release -- filter "Does this file parse command line arguments?" --read-files --max-cost 0.10` and save stdout and the stderr summary line to `scratch/readme-run.txt`; if no key is available to the agent, hand the command to the maintainer and wait for the output. Also capture one real `map` output line for the examples section the same way (a `-q` with a noul and a choice question over a few lines of a log or `git log --oneline -5`)
- [X] T013 [US4] Write `README.md` (contracts/readme.md, R8, FR-020–FR-022, FR-020b): three badges (CI workflow badge for `ci.yml`, PyPI version `https://img.shields.io/pypi/v/jevpipe`, license); the nine sections in the contract's order; the first screen shows the command and the real output from T012; install is `uv tool install jevpipe` with one line each for `pipx install jevpipe`, uv's own installer (`curl -LsSf https://astral.sh/uv/install.sh | sh`, `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`), `uv tool update-shell` and `uv tool upgrade jevpipe`; the skill section with `npx skills add fabianboth/jevpipe` and `gh skill install fabianboth/jevpipe jevpipe`; `jq` recommended for `map` with its link `https://jqlang.org/download/`; every link and image absolute (`https://github.com/fabianboth/jevpipe/blob/main/LICENSE`, …), no GitHub alerts, no `<picture>`, under ~120 lines
- [X] T014 [US4] Extend `tests/cli/docs.rs` (R10, FR-019, FR-020b): `readme_flags_exist_in_the_help` (as T011 for `README.md`); `readme_has_no_relative_links` (every Markdown link or image target `](…)` in `README.md` starts with `https://` or `#`; failure names the target)
- [X] T015 [P] [US4] In `.coderabbit.yaml` `reviews.path_filters`, add `README.md`, `pyproject.toml` and `LICENSE`; add a `path_instructions` entry for `README.md`: "Flag any command, flag or output shape that does not match the CLI, and any relative link (the README is also the PyPI description)"

**Checkpoint**: `./check.ps1 -Fix` passes; the README reads top to bottom in under three minutes.

---

## Phase 7: User Story 5 - The repository is safe to make public (Priority: P3)

**Goal**: personal settings out of the repository, one stable required CI check.
(Already done on 2026-09-27: history scan, public switch, secret scanning, push protection,
read-only default token.)

**Independent Test**: the repository's `.claude/settings.json` has no `hooks` and unchanged
permissions; `.vscode/settings.json` has only the clippy setting; the PR shows a `ci-success` check.

- [X] T016 [P] [US5] Move the notify hooks out of the repository (FR-023, quickstart §1.3): (revised: project-local, not user-level) put the hooks into the gitignored `.claude/settings.local.json`, keep `.claude/scripts/notify.ps1` locally but untracked via `.git/info/exclude`; originally: copy `.claude/scripts/notify.ps1` to `~/.claude/scripts/notify.ps1`; in `~/.claude/settings.json` add the `PermissionRequest` and `Stop` hook entries from the repository's `.claude/settings.json` to the existing `hooks` object, next to the existing `SubagentStart` entry, with the command path changed to `$HOME/.claude/scripts/notify.ps1` (check how the existing entry quotes paths and follow it); leave every other key of that file untouched and show the maintainer the diff; then remove the whole `hooks` key from `.claude/settings.json` (permissions unchanged) and delete `.claude/scripts/notify.ps1` (and the empty `.claude/scripts/` folder)
- [X] T017 [P] [US5] Reduce `.vscode/settings.json` to `{ "rust-analyzer.check.command": "clippy" }` (FR-024)
- [X] T018 [P] [US5] In `.github/workflows/ci.yml` add the job `ci-success` (R9): `needs: check`, `if: always()`, `runs-on: ubuntu-latest`, one step that exits 1 unless `needs.check.result == 'success'` (e.g. `run: test "${{ needs.check.result }}" = success`)

**Checkpoint**: `./check.ps1 -Fix` passes.

---

## Phase 8: Polish, merge and release

**Purpose**: finish, merge, protect `main`, release v0.1.0, and check the skill for real.

- [X] T019 Confirm `specs/manual/idea-draft.md` "Next" matches the final plan (PyPI only, one check, no release candidate) and adjust if not
- [X] T020 Run `./check.ps1 -Fix` and the quickstart §1 checks (local wheel T004 again with the final README, environments, notify hooks); commit per phase with the attribution from the session's system reminder
- [ ] T021 Push the branch, open the PR (title "Release 0.1.0 on PyPI and add the agent skill"), wait for CI including `ci-success`; after review, merge to `main` **(maintainer approves the merge)**
- [ ] T022 Create the ruleset on `main` (R9, quickstart §2.2) with `gh api -X POST repos/fabianboth/jevpipe/rulesets`: target `branch`, `~DEFAULT_BRANCH`, enforcement `active`, rules `deletion`, `non_fast_forward`, `pull_request` (0 approvals, all merge methods), `required_status_checks` with `ci-success` (integration id 15368), no bypass actors; verify a direct push to `main` is refused (quickstart §2.3)
- [ ] T023 **(maintainer)** Confirm T007 is done, then tag the release on `main`: `git tag v0.1.0 && git push origin v0.1.0`; watch with `gh run watch`; on a failure before the TestPyPI upload, fix via PR and move the tag; after it, fix and release the next patch version (quickstart §3)
- [ ] T024 **(maintainer)** When TestPyPI is green, check https://test.pypi.org/project/jevpipe/ (README, links) and approve the `pypi` deployment; afterwards confirm https://pypi.org/project/jevpipe/ shows 0.1.0 with attestations and the GitHub Release `v0.1.0` exists with the six wheels
- [ ] T025 Check the skill install (quickstart §4.1): in a temporary directory under `scratch/`, `npx skills add fabianboth/jevpipe` (choose Claude Code) installs only `SKILL.md`, under 50 KB; `gh skill install fabianboth/jevpipe jevpipe` (if gh ≥ 2.90) likewise; remove both afterwards
- [ ] T026 **(maintainer, costs a few cents)** The agent test of quickstart §4.2 and the README test of quickstart §5; record surprises in `specs/manual/skill-learnings.md` for 005

---

## Dependencies & Execution Order

- **Setup (T001–T002)** first.
- **US1 (T003–T004)** → **US2 (T005–T008)**: the workflow builds what `pyproject.toml` describes.
- **US3 (T009–T011)** and **US5 (T016–T018)** are independent of US1/US2 and of each other.
- **US4 (T012–T015)**: T013 needs T012's real output; T014 extends the file T011 created, so after
  T011. T004's final run needs the README.
- **Phase 8**: T020–T021 after every story; T022 after the merge; T023 after T007 and T022;
  T024 after T023; T025–T026 after T021 (the skill) and T024 (the binary).

## Parallel Opportunities

- T001 ∥ T002.
- After Setup: US1/US2 ∥ US3 ∥ US5 (T016 ∥ T017 ∥ T018).
- T015 ∥ T013/T014.

## Implementation Strategy

1. Setup, then US1 (the wheel works locally), then US2 (the pipeline exists, environments set,
   publisher values handed to the maintainer early, since registering them takes the maintainer's
   time).
2. US3 and US5 in parallel, then US4 (needs a real run and the skill's test file).
3. Merge, protect `main`, tag v0.1.0, approve, check the skill.

MVP: Setup + US1 + US2 + the README (US4) is a usable public release; the skill (US3) is what makes
it reach coding agents, so it ships in the same release.

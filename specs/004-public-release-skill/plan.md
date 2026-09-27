# Implementation Plan: First Public Release and the Agent Skill

**Branch**: `004-public-release-skill` | **Date**: 2026-09-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/004-public-release-skill/spec.md`

## Summary

No product code changes. Five pieces around the existing binary:

1. **Release pipeline.** One workflow, `release.yml`, on a `vMAJOR.MINOR.PATCH` tag: maturin builds
   six binary-only wheels (static musl on Linux, tagged for glibc and musl; native `windows-11-arm`);
   each is installed with uv and run on its platform; all go to TestPyPI as a trial run; after the
   maintainer's approval they go to PyPI with attestations; finally a GitHub Release with notes and
   the wheels. Installed with `uv tool install jevpipe`. No npm, no install scripts, no pre-releases.
2. **Agent skill.** `skills/jevpipe/SKILL.md`, written with the skill-creator guidance.
3. **README and license.** A landing-page README with absolute links (it is also the PyPI
   description), Apache-2.0 LICENSE, `license` and `repository` in `Cargo.toml`, `pyproject.toml`.
4. **Docs drift test.** An offline test: every `--flag` in the README and the skill exists in the
   binary's help, the skill's name matches its folder, the README has no relative links.
5. **Protecting the public repository.** Personal settings leave it; `ci.yml` gains the aggregating
   `ci-success` job; environments `testpypi` and `pypi` are created; after the merge a ruleset
   protects `main`; then `v0.1.0` is tagged.

## Technical Context

**Language/Version**: Rust 1.98 (edition 2024), pinned in `rust-toolchain.toml`; GitHub Actions YAML;
TOML; Markdown
**Primary Dependencies**: no new crates. Release tooling: maturin 1.15 via `PyO3/maturin-action`
v1.51.0, `pypa/gh-action-pypi-publish` v1.14.2, `astral-sh/setup-uv`, the preinstalled `gh`
**Storage**: N/A
**Testing**: `cargo test` (new `tests/cli/docs.rs`, offline, real binary); in each release run every
wheel is installed with uv and run on its own platform before any upload; a local `maturin build`
before the first tag; manual quickstart for the skill
**Target Platform**: `{x86_64,aarch64}-{apple-darwin,unknown-linux-musl,pc-windows-msvc}`
**Project Type**: single CLI crate plus a release workflow, packaging metadata, agent skill and docs
**Performance Goals**: tag to TestPyPI under 30 minutes (SC-002); installed command starts like the
bare binary (SC-002a, measured 28 ms both)
**Constraints**: no stored publishing token; tests never touch the network; skill folder text only,
under 50 KB; README under ~120 lines with absolute links only
**Scale/Scope**: ~12 new or changed files in the repository, no `src/` changes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Lean MVP | Pass: one channel (PyPI via uv) with one build tool and one workflow; one check per wheel; no npm, install scripts, pre-releases, Homebrew, crates.io, platform signing or updater; the skill is one file; the README a landing page; only the repository settings that protect a public repository |
| II. Automated Verification | Pass: the drift test keeps README and skill honest offline; every release installs and runs each wheel on its own platform before anything is uploaded; `check.ps1` unchanged and passing |
| III. Reusable Components | Pass: one `check` matrix job serves all six platforms; the drift test reuses the `tests/cli` harness |

Post-design re-check: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/004-public-release-skill/
├── spec.md
├── plan.md
├── research.md          # R1–R10: channel, wheels and runners, packaging, release workflow, smoke test, versions and tags, skill format, README, going public, drift test
├── data-model.md
├── quickstart.md        # the runbook: maintainer setup, merge and protect, v0.1.0, skill check
├── contracts/
│   ├── release.md       # tag → wheels, check, TestPyPI, approval, PyPI, GitHub Release
│   ├── skill.md         # SKILL.md frontmatter and sections
│   └── readme.md        # README sections, order and rules
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml                         # CHANGED: license = "Apache-2.0", repository
pyproject.toml                     # NEW: maturin bin wheel metadata (research R3)
LICENSE                            # NEW: Apache License 2.0
README.md                          # NEW: contracts/readme.md; also the PyPI description
skills/jevpipe/SKILL.md            # NEW: contracts/skill.md
.github/workflows/
├── ci.yml                         # CHANGED: ci-success aggregating job
└── release.yml                    # NEW: research R4, R5
.claude/settings.json              # CHANGED: notify hooks removed; permissions unchanged
.claude/scripts/notify.ps1         # UNTRACKED: stays locally, excluded via .git/info/exclude
.vscode/settings.json              # CHANGED: only rust-analyzer.check.command
.coderabbit.yaml                   # CHANGED: path filters gain README.md, pyproject.toml, LICENSE
CLAUDE.md                          # CHANGED: releases (version tags, release.yml, pyproject.toml; README doubles as the PyPI description)
specs/manual/idea-draft.md         # CHANGED: Next section

tests/cli/
├── main.rs                        # CHANGED: mod docs
└── docs.rs                        # NEW: flags exist, skill name matches folder, no relative README links

.claude/settings.local.json        # NOT COMMITTED (gitignored): the maintainer's notify hooks
```

GitHub settings (via `gh api`): environments `testpypi` and `pypi` (research R4), the ruleset on
`main` after the merge (research R9). On pypi.org and test.pypi.org the maintainer registers the
pending publishers (research R4).

**Structure Decision**: no product code changes, so no new modules. The skill lives where CLAUDE.md
and the installers expect it (`skills/jevpipe/`). `pyproject.toml` sits next to `Cargo.toml`, where
maturin expects it.

## Design

1. **Check the files, once, before any upload.** Each wheel is installed with uv from the built files
   on its own runner and run; TestPyPI and PyPI then receive exactly those files.
2. **One gate, in the right place.** Everything up to TestPyPI runs unattended; the `pypi` job waits
   for the maintainer's approval; the GitHub Release is created only after PyPI succeeded, so a
   release on GitHub always means a release on PyPI.
3. **Only version tags publish.** The workflow triggers on `v[0-9]+.[0-9]+.[0-9]+`, the environments
   allow only `v[0-9]*.[0-9]*.[0-9]*` tags, and the first step checks the tag against `Cargo.toml`.
4. **Publishing jobs hold the only write tokens.** `permissions: {}` at the top; `contents: read`
   for build and check jobs; `id-token: write` only for the two upload jobs; `contents: write` only
   for `github-release`; `persist-credentials: false`; third-party actions pinned by SHA.
5. **The docs test stays simple.** It checks flags against the combined help rather than parsing
   each shell line into a subcommand path.
6. **README output is real.** The first-screen example and any shown output come from a real run
   against this repository with the maintainer's key (a few cents), not from invention.
7. **The skill states guidance as guidance.** Thresholds, phrasing advice and the review band are
   marked as current guidance until 005 measures them; the skill names 0.1.0 and points to `--help`.
8. **Rulesets need one stable check.** `ci-success` aggregates the matrix so the ruleset never
   depends on matrix-derived names.

## Complexity Tracking

No violations.

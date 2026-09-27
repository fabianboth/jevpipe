# Specification Quality Checklist: First Public Release and the Agent Skill

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-27
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- For a release milestone the distribution channels (PyPI, GitHub Releases, the
  agent-skill installers) and the repository settings are the product's surface, so naming them is
  behaviour, not implementation. maturin appears only in the assumptions as the chosen tool; the
  plan confirms it and settles targets, the PyPI step and the drift test.
- Decided with the user (2026-09-27): six targets (Linux, macOS, Windows on x64 and arm64); npm (later
  replaced by PyPI, see below) reserved with a stage-only trusted publisher and 2FA-only access set
  up on npmjs.com; no Homebrew or crates.io; Apache-2.0; the skill carries no install steps, only a
  README link; release and skill in one milestone, calibration and examples in 005; the repository goes
  public at the end of this milestone with its AI tooling and specs; shared permissions stay broad,
  only the notify hooks leave the repository; the "not affiliated" statement goes into the README.
- Revised with the user while planning (2026-09-27): PyPI (binary wheels via uv/pipx) replaces npm,
  because npm always starts a Node.js launcher; GitHub build attestations for the binaries; the
  release candidate runs after the switch to public; the aggregating CI job is named `ci-success`;
  no README note on manual browser downloads; the PR threads need no review before going public.
- Decided with the user (2026-09-27): releases run on version tags; installs are checked by CI on all
  six platforms, not by hand (FR-010a); the skill is written with the `skill-creator` skill (FR-017a).
- Proposed, awaiting the user: `check.ps1` stays PowerShell 7, which runs on Linux and macOS as well.
- Decided with the user (2026-09-27, revised during implementation): the notify hooks move to the
  project-local, gitignored `.claude/settings.local.json`; the script stays at
  `.claude/scripts/notify.ps1`, untracked through `.git/info/exclude`.
- Revised again with the user (2026-09-27): PyPI is the only channel for 0.1.0; install scripts
  (cargo-dist) are deferred, so there is one build tool (maturin) and one workflow, `release.yml`,
  with TestPyPI as staging, an approval gate before PyPI and the GitHub Release created last. This
  supersedes the attestation and install-script parts of the earlier revision.
- Simplified with the user (2026-09-27): one install check per wheel on its own platform before any
  upload (no repeated checks from TestPyPI and PyPI, no Alpine container, no composite action); no
  pre-releases and no release candidate (the first release is 0.1.0; only `vMAJOR.MINOR.PATCH` tags
  release, enforced by the workflow filter and the environments' tag policy); the smoke test is
  `--version` plus a run without a key; the drift test checks flags against the combined help.

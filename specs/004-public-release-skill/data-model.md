# Data Model: First Public Release and the Agent Skill

No runtime data changes; jevpipe's records, answers, config and key are as in 003. The entities
below are release and repository artifacts.

## Release

- **Identity**: tag `vMAJOR.MINOR.PATCH`, equal to `package.version` in `Cargo.toml`; no
  pre-releases.
- **Contents**: six binary-only wheels (no sdist) with PyPI attestations; a GitHub Release with
  generated notes and the same wheels.
- **States**: tagged → built (all six or nothing) → checked on each platform → on TestPyPI →
  approved → on PyPI → GitHub Release. A failure stops the chain; nothing reaches PyPI without
  approval, and no GitHub Release exists without a PyPI release. A run that fails before the TestPyPI
  upload is fixed and the tag set again; after it, the fix ships as the next patch version.

## Platform

| Platform | Target | Build runner | Check runner |
|---|---|---|---|
| Linux x64 | `x86_64-unknown-linux-musl` | `ubuntu-latest` (musllinux container) | `ubuntu-latest` |
| Linux arm64 | `aarch64-unknown-linux-musl` | `ubuntu-latest` (musllinux container) | `ubuntu-24.04-arm` |
| macOS x64 | `x86_64-apple-darwin` | `macos-15-intel` | `macos-15-intel` |
| macOS arm64 | `aarch64-apple-darwin` | `macos-14` | `macos-latest` |
| Windows x64 | `x86_64-pc-windows-msvc` | `windows-2022` | `windows-latest` |
| Windows arm64 | `aarch64-pc-windows-msvc` | `windows-11-arm` | `windows-11-arm` |

## Agent skill

- **Folder**: `skills/jevpipe/`, containing only `SKILL.md`.
- **Frontmatter**: `name` (= folder name), `description` (1–1024 chars), `license`,
  `compatibility`, `metadata.jevpipe-version`.
- **Version relation**: installed independently of the binary; `metadata.jevpipe-version` names the
  release it was written for. `gh skill install` without a version takes the latest tagged release;
  `npx skills` takes the default branch.

## Repository settings

- Done: public; secret scanning and push protection on; default workflow token read-only.
- Environments: `testpypi` and `pypi`, both limited to tags `v[0-9]*.[0-9]*.[0-9]*`; `pypi` with the
  maintainer as required reviewer.
- After the merge: ruleset `main` on the default branch: no deletion, no force push, pull request
  required (0 approvals), required status check `ci-success`; no bypass actors; tags unaffected.
- Outside GitHub: pending trusted publishers for `jevpipe` on pypi.org and test.pypi.org
  (`fabianboth/jevpipe`, `release.yml`, environment `pypi` / `testpypi`); npm `jevpipe@0.0.0` stays
  as the name reservation.

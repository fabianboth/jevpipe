# Contract: Release

## Trigger

A pushed tag matching `v[0-9]+.[0-9]+.[0-9]+` (e.g. `v0.1.0`) starts `release.yml`; the version must
equal `package.version` in `Cargo.toml`. Other tags, branch pushes and pull requests start no release
(`ci.yml` runs as before).

## release.yml (a failure stops every later job)

| Job | Runs on | Does | Needs |
|---|---|---|---|
| build (×6) | the build runners (data-model) | tag equals `Cargo.toml` version; `maturin build --release --locked`; Linux: musl, tagged `manylinux_2_17` + `musllinux_1_1` | – |
| check (×6) | the check runners (data-model), each with its own platform's wheel | `uv tool install --managed-python --no-index --find-links wheels jevpipe`, then the smoke test | build |
| testpypi | ubuntu | upload all wheels to TestPyPI, `skip-existing`; environment `testpypi` | check |
| pypi | ubuntu | upload all wheels to PyPI with attestations; environment `pypi` (**required reviewer**) | testpypi |
| github-release | ubuntu | `gh release create v<version> --verify-tag --generate-notes` with the wheels | pypi |

Wheels: `jevpipe-<version>-py3-none-<platform>.whl`, six files, no sdist.

Permissions: `{}` at the top; build and check jobs `contents: read`; `testpypi` and `pypi`
`id-token: write`; `github-release` `contents: write`. Checkouts with `persist-credentials: false`;
third-party actions pinned by SHA.

## Smoke test (in `check`)

With `OPENROUTER_API_KEY` unset:

1. `jevpipe --version` prints `jevpipe <version>`
2. `echo x | jevpipe filter "q"` exits 2 and standard error names `OPENROUTER_API_KEY` (the "no API
   key" message where a keychain exists, the "no keychain" message where none does)

## Installing (user side)

| | `uv tool install jevpipe` | `pipx install jevpipe` |
|---|---|---|
| Installs | the native binary into `~/.local/bin` (`%USERPROFILE%\.local\bin`); a symlink on Linux | the native binary into pipx's bin directory |
| Integrity | wheel hash verified; PyPI attestations | wheel hash verified |
| PATH | uv warns if the directory is missing; `uv tool update-shell` adds it | `pipx ensurepath` |
| Upgrade | `uv tool upgrade jevpipe` | `pipx upgrade jevpipe` |
| Unsupported platform | "no matching distribution", nothing installed | same |

## Settings outside the workflow

- Environments: `testpypi` (deployment tag policy `v[0-9]*.[0-9]*.[0-9]*`), `pypi` (reviewer: the
  maintainer, `prevent_self_review: false`; the same tag policy).
- Pending trusted publishers on pypi.org and test.pypi.org: owner `fabianboth`, repository
  `jevpipe`, workflow `release.yml`, environment `pypi` / `testpypi`, project `jevpipe`.
- `ci.yml` gains `ci-success`: `needs: check`, `if: always()`, fails unless every `check` job
  succeeded. The ruleset on `main` requires `ci-success` only.

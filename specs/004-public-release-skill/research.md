# Research: First Public Release and the Agent Skill

Research passes on 2026-09-27: release channels (a cargo-dist prototype in `scratch/distproto/`, a
maturin prototype in `scratch/pypiproto/`), npm, the Agent Skills format, reference READMEs, GitHub
settings and runners, and the maintainer's existing PyPI projects (`reviewloop`, `ralph-ideate`).
**[P]** marks what a prototype verified.

## R1. Channel: PyPI only

- **Decision**: one channel, binary-only wheels on PyPI, installed with `uv tool install jevpipe`
  (or `pipx install jevpipe`). A GitHub Release per version carries notes and the same wheels.
- **Why PyPI** [P]: the wheel holds the native executable; `uv tool install` puts that exact file on
  the path (same SHA-256 as `target/release/jevpipe.exe`; a symlink on Linux), `--version` in 28 ms
  like the bare binary (`uvx --from` warm: 67 ms). ruff and uv ship this way. Trusted publishing,
  PyPI attestations, per-file hashes, `uv tool upgrade`, and an approval gate are the trust and
  update story the maintainer wants; the audience (developers with coding agents) mostly has uv.
- **Why not npm**: every Rust tool on npm (Biome, Codex CLI, oxlint) starts through a Node.js
  launcher (~40 ms per call measured here); the only trick that avoided it swaps the launcher in an
  install script, and npm 12 (July 2026) blocks install scripts by default, also for global installs.
  The name `jevpipe` stays reserved on npm.
- **Why no install scripts (cargo-dist) for now**: they would add a second tool, a second build of
  every platform and a second check matrix, for users without uv, who can install uv with one command.
  cargo-dist can be added later as a second channel without changing anything for uv users. The
  prototype showed what that would take: `dist = true` and `repository` in the config, musl targets,
  a native `windows-11-arm` runner, `install-path = "~/.local/bin"`.
- **Costs accepted**: users without uv run two commands; uv downloads a Python once if the machine
  has none (its tool environment needs one, jevpipe never uses it); no bare binary download.

## R2. Wheels and runners

- **Decision**: six wheels, built with `PyO3/maturin-action` (v1.51.0, maturin 1.15) on a `v*` tag:
  | Platform | Target | Build runner |
  |---|---|---|
  | Linux x64 | `x86_64-unknown-linux-musl` | `ubuntu-latest` (musllinux container) |
  | Linux arm64 | `aarch64-unknown-linux-musl` | `ubuntu-latest` (musllinux container) |
  | macOS x64 | `x86_64-apple-darwin` | `macos-15-intel` |
  | macOS arm64 | `aarch64-apple-darwin` | `macos-latest` |
  | Windows x64 | `x86_64-pc-windows-msvc` | `windows-latest` |
  | Windows arm64 | `aarch64-pc-windows-msvc` | `windows-11-arm` |
- **Linux**: static musl builds tagged both `manylinux_2_17` and `musllinux_1_1` (uv's approach:
  `manylinux: musllinux_1_1`, `--compatibility pypi --compatibility 2_17`), so one wheel serves glibc
  and musl distributions [P: tags produced; installed and ran on glibc Ubuntu 22.04 with uv and pip].
  reqwest 0.13's default TLS is rustls with aws-lc-rs; aws-lc-sys needs only a C compiler, builds in
  these containers (uv does the same) and its musl breakage was fixed in 0.32.1. zbus is pure Rust.
- **Windows arm64**: native on `windows-11-arm` (maturin-action downloads an arm64 maturin); a
  cross-build in a container has known aws-lc breakage.
- **Toolchain**: maturin-action honours `rust-toolchain.toml` (Rust 1.98).
- **Risk**: the existing CI tests glibc x64 builds only; the first release run exercises musl and
  the arm runners before anything is uploaded. Intel macOS runners end in August 2027.

## R3. Packaging (`pyproject.toml`)

- [P] At the repository root: `build-backend = "maturin"` (`requires = ["maturin>=1.15,<2"]`),
  `[tool.maturin] bindings = "bin"`; `[project]` with `name = "jevpipe"`,
  `dynamic = ["version"]` (taken from `Cargo.toml`), `description`,
  `readme = "README.md"`, `license = "Apache-2.0"`, `license-files = ["LICENSE"]`,
  `requires-python = ">=3.8"`, keywords, classifiers, `[project.urls]` Repository. No
  `strip` there: the release profile in `Cargo.toml` already strips.
- maturin ignores `publish = false`; cargo-deny and `check.ps1` never read `pyproject.toml`. The wheel
  holds `jevpipe-<v>.data/scripts/jevpipe[.exe]`, METADATA, a CycloneDX SBOM and the license.
- **No sdist**: an unsupported platform gets a clean "no matching distribution" instead of an
  attempted Rust and C build.
- **One README** [P]: `README.md` is the PyPI description. PyPI leaves relative links and images
  relative (broken) and shows GitHub alerts as a literal `[!NOTE]`; ruff and uv use a single README
  with absolute URLs. So: absolute `https://github.com/fabianboth/jevpipe/...` links, no alerts, no
  `<picture>`, no separate `README_PYPI.md` (unlike the maintainer's earlier projects).

## R4. Release workflow (`release.yml`)

- **Decision**: one top-level workflow on tags matching `v[0-9]+.[0-9]+.[0-9]+`, modelled on the
  maintainer's earlier projects:
  1. `build` (matrix of six): check the tag equals `Cargo.toml`'s version; build the wheel.
  2. `check` (six runners): install from the built wheels with
     `uv tool install --no-index --find-links wheels jevpipe`, then the smoke test (R5).
  3. `testpypi` (environment `testpypi`): upload with `pypa/gh-action-pypi-publish` to
     `https://test.pypi.org/legacy/`; the trial run of the upload itself. No `skip-existing`: it would
     let a rerun keep older TestPyPI files while newly built ones go to PyPI.
  4. `pypi` (environment `pypi`, **required reviewer**): upload with `pypa/gh-action-pypi-publish`.
  5. `github-release`: `gh release create <tag> --verify-tag --generate-notes` with the wheels.
- **One check, before any upload**: TestPyPI and PyPI serve the same bytes, so installing again from
  them would test the same files; the check on each platform from the built files covers what
  matters (uv picks the right wheel, it installs, it runs).
- **Why the pypa action, not `uv publish`** (used in the earlier projects): it creates PEP 740
  attestations automatically under trusted publishing; `uv publish` only uploads existing
  attestation files.
- **Why top-level**: trusted publishing from a reusable workflow is officially unsupported
  (warehouse#11096).
- **Permissions**: `permissions: {}` at the top; build and check jobs `contents: read`; only
  `testpypi` and `pypi` get `id-token: write`; `github-release` gets `contents: write`. Checkouts use
  `persist-credentials: false`; third-party actions are pinned by SHA.
- **Improvements over the earlier projects**: a required reviewer and a version-tag policy on the
  environments (theirs have none, so every tag goes straight to PyPI); attestations; installing and
  running every wheel on its platform before the upload; SHA pinning; the GitHub
  Release only after PyPI succeeded.
- **Environments** (via `gh api`; required reviewers are free on public repositories): `pypi` with the
  maintainer as reviewer (`prevent_self_review: false`) and a deployment tag policy
  `v[0-9]*.[0-9]*.[0-9]*`; `testpypi` with the tag policy only. Together with the workflow's tag
  filter, only proper version tags can publish.
- **Pending publishers** (the maintainer registers them): on pypi.org and test.pypi.org, owner
  `fabianboth`, repository `jevpipe`, workflow `release.yml`, environment `pypi` / `testpypi`,
  project `jevpipe`. They do not reserve the name before the first upload.

## R5. The smoke test

- **Decision**: in the `check` job, with `OPENROUTER_API_KEY` unset, on `ubuntu-latest`,
  `ubuntu-24.04-arm`, `macos-15-intel`, `macos-latest`, `windows-latest`, `windows-11-arm`:
  1. `jevpipe --version` prints `jevpipe <version>`
  2. `echo x | jevpipe filter "q"` exits 2 and standard error names `OPENROUTER_API_KEY` (both the
     "no API key" and the "no keychain" message do); this runs the platform's keychain lookup in the released binary, the
     part the regular CI does not cover on arm64 and musl.
- Not added: an Alpine container check (the Linux wheels are static musl builds, so they run on
  Alpine; only uv's tag selection would be tested, which is standard).

## R6. Versions and tags

- Only tags `vMAJOR.MINOR.PATCH` release; no pre-releases. The first release is 0.1.0 itself: a
  failure publishes nothing to PyPI. Before the TestPyPI upload, the fix is followed by setting the
  tag again; once TestPyPI has the version (its files can never be replaced), the fix ships as the
  next patch version, so TestPyPI always staged exactly the files that reach PyPI.
- The first step of `build` fails the run when the tag and `Cargo.toml` differ.
- `gh skill install` without a version installs the skill from the latest tagged release;
  `npx skills` follows the default branch.

## R7. Agent skill format

- **Spec** (agentskills.io): `name` 1–64 chars `[a-z0-9-]`, equal to the folder name; `description`
  ≤ 1024 chars, third person, what and when; optional `license`, `compatibility` (≤ 500 chars),
  `metadata` (string map). Body under 500 lines / ~5000 tokens; `references/` only when needed.
- **skill-creator guidance**: the description is the trigger and should be "a little bit pushy"
  (use it whenever …, even if jevpipe is not named); it names multi-step bulk tasks, since simple
  one-step tasks rarely trigger a skill; imperative voice, examples, explain *why* instead of
  capital-letter rules; state only what the model lacks; one default plus an escape hatch.
- **Installers**: `npx skills add fabianboth/jevpipe --skill jevpipe` (`-g` for all projects) and
  `gh skill install fabianboth/jevpipe jevpipe` (gh ≥ 2.90, `--scope user`) both find
  `skills/*/SKILL.md` and copy only that folder; a root `SKILL.md` would shadow it. `npx skills` also
  finds the development skills in `.claude/skills/` (spec-kit, reviewloop: 16 in all), so the
  documented command names the skill with `--skill jevpipe`; marking those skills `internal: true`
  would be undone by every spec-kit upgrade.
- **Decision**: no `allowed-tools` (every run spends the user's credit, so the user keeps approving
  runs); `metadata.jevpipe-version: "0.1.0"`; no `references/` for now (~200 lines). Comparable skills
  (gh-skill, agent-browser) do not handle secrets; the key rule is new ground.

## R8. README shape

- **Studied**: jq, uv, llm, fd, ripgrep, bat, gum, agent-browser, and the maintainer's `reviewloop`.
  The short, loved READMEs are landing pages: name, one sentence, a real run, install, a quick start,
  links to depth, license; depth lives in `--help`. None has a cost or "not affiliated" statement, so
  ours are one plain sentence each.
- **Rules**: one sentence and a real command with its real output on the first screen; commands in
  fenced blocks without `$`, output in separate blocks; install is `uv tool install jevpipe` with
  pipx and uv's own installer as one-liners; the key in one command right after install; two or three
  one-line examples, then "see `--help`"; the skill in its own section with one command; under ~120
  lines; three badges (CI, PyPI version, license, as in `reviewloop`); no table of contents; absolute
  links only (R3).

## R9. Going public (done 2026-09-27) and protecting `main`

- **Done**: gitleaks 8.30.1 over all refs found only the fake `sk-or-v1-secret-9f8e7d` in
  `tests/auth.rs`; the maintainer switched the repository to public; secret scanning and push
  protection are enabled; the default workflow token is read-only.
- **Protecting `main`** (after this feature is merged): a repository ruleset on `~DEFAULT_BRANCH`:
  no deletion, no force push, pull request required with 0 approvals (a sole maintainer cannot
  approve their own), required status check `ci-success`; no bypass actors; tags unaffected. Matrix
  check names contain every matrix value, so `ci.yml` gains the aggregating job `ci-success` (needs
  `check`, `if: always()`, fails unless all succeeded) and only that is required.
- **Deferred**: private vulnerability reporting with `SECURITY.md`, Dependabot, CodeQL.

## R10. Keeping docs and CLI in step

- **Decision**: an integration test (`tests/cli/docs.rs`) with three small checks:
  1. every `--flag` word in `README.md` and `skills/*/SKILL.md` appears in the combined `--help`
     output of the real binary, collected from the top level down through every `Commands:` list.
     Only flags in a pipe segment that runs `jevpipe`, or in a code span that starts with `--`, count,
     so other tools' flags (`rg --files`, `git log --format=%s`) are left alone;
  2. the skill's frontmatter `name` equals its folder name;
  3. the README has no relative Markdown links or images (R3).
- **Rationale**: offline, with the existing `assert_cmd` harness; catches a renamed or removed flag
  and a link that would break on PyPI (FR-019, FR-020b, SC-006). Matching each flag to its own
  subcommand would need a shell-line parser, for little extra safety.

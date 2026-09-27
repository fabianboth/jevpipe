# Feature Specification: First Public Release and the Agent Skill

**Feature Branch**: `004-public-release-skill`
**Created**: 2026-09-27
**Status**: Draft
**Input**: User description: "jevpipe 004: first public release and the agent skill. See conversation for all decisions: prebuilt binaries for Linux, macOS and Windows (x64 and arm64) built by cargo-dist on a version tag and attached to a GitHub Release, with shell and PowerShell install scripts that pick the right binary; npm package `jevpipe` (name already reserved with a 0.0.0 placeholder by the npm account bothlabs; trusted publisher for fabianboth/jevpipe with workflow release.yml already configured, direct publish not allowed) staged from CI through npm trusted publishing with no stored token (npm stage publish, npm CLI 11.15.0+) and approved by hand with 2FA; no Homebrew, no crates.io (publish = false stays). Agent skill at skills/jevpipe/SKILL.md following the Agent Skills spec, installable separately through `npx skills add fabianboth/jevpipe` or `gh skill install`, carrying no binary and no install steps, only a pointer to the README when jevpipe is missing; it teaches when to reach for jevpipe (bulk or repetitive judgments), when not (exact grep, arithmetic, generated text), how to phrase questions, thresholds, limits (--max-cost, exit status 3) and that the agent never handles the API key (the user runs `jevpipe auth set-key` or sets OPENROUTER_API_KEY); written from specs/manual/skill-learnings.md and current knowledge, calibration and evals are 005. A short, simple README modelled on good reference repos (what it is, install, key, two or three examples, installing the skill, cost warning, unofficial and not affiliated with TypeSafe AI, license). License Apache-2.0 with the license field in Cargo.toml. Cleanup before going public: notify hooks move out of the shared .claude/settings.json into the user's own settings (broad permissions stay as they are), Peacock colors removed from .vscode/settings.json (the clippy setting stays). Going public: tried first on the private repo with a pre-release tag (the release CI shows whether every target builds, including the Linux keychain and TLS dependencies), then the repo is switched to public with secret scanning and push protection and a protected main branch, v0.1.0 is tagged, and installing through the script and npm is checked. The calibration study and the example pipelines are out of scope (005)."

## Context

jevpipe works, but only for someone who clones the private repository and builds it with the pinned
Rust toolchain. This milestone turns it into something a stranger and a coding agent can use within
minutes: a public repository, prebuilt packages for every mainstream platform, a one-line install,
a README that gets a first run going, and the agent skill that teaches coding agents when and how to
reach for jevpipe.

The skill and the binary are distributed separately, as other command-line tools with agent skills do
(the GitHub CLI, agent-browser). The skill is a small text folder installed by the agent-skill
installers (`npx skills add`, `gh skill install`), which copy the folder that holds `SKILL.md` and
nothing else; the binary comes from the release. The skill therefore carries no binary and no install
steps, only a pointer to the README.

Revised while planning (2026-09-27, with the user): jevpipe ships through one channel, PyPI, installed
with `uv tool install jevpipe` (or `pipx`). A binary-only wheel installs the native executable itself,
the way ruff and uv ship, with trusted publishing, signed attestations and an approval gate. npm is
not a channel: a Rust binary on npm always starts through a Node.js launcher (npm 12 no longer runs
the install scripts that could avoid it); its name stays reserved. Standalone install scripts
(cargo-dist) are deferred: they would add a second build and toolchain for users without uv, who
can install uv with one command; they can be added later without changing anything for existing
users. The repository is already public, and the first release is 0.1.0 itself: nothing reaches PyPI
before every check and the maintainer's approval, so no separate release candidate is needed.

The result is version 0.1.0. How well the decisions match reality (calibration, thresholds, which
questions beat grep) is measured in 005, which then revises the skill's advice and the defaults.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install jevpipe with one command (Priority: P1)

A developer on Linux, macOS or Windows, on an x64 or arm64 machine, installs jevpipe with one command
from the README: `uv tool install jevpipe` (or `pipx install jevpipe`); without uv, the README's
one-line uv install comes first. uv picks the package for their platform and puts the `jevpipe`
command on their path. They run `jevpipe --version` and see 0.1.0. No Rust toolchain or compiler is
needed, and nothing but the native binary runs when they call `jevpipe`.

**Why this priority**: Without it nobody outside this machine can use jevpipe; every other part of
this milestone assumes an installed binary.

**Independent Test**: After the v0.1.0 release, on a fresh runner of each supported platform, run
`uv tool install jevpipe` and then `jevpipe --version`; it prints 0.1.0.

**Acceptance Scenarios**:

1. **Given** a machine of a supported platform with uv, **When** the user runs `uv tool install jevpipe`, **Then** the package for that platform is downloaded with its hash verified, the command on the path is the native binary itself (no Python launcher), and `jevpipe --version` prints the released version.
2. **Given** a machine with pipx, **When** the user runs `pipx install jevpipe`, **Then** the same holds.
3. **Given** jevpipe 0.1.0 installed, **When** the user runs `uv tool upgrade jevpipe` after a later release, **Then** the new version replaces the old binary; the user's config file and stored API key are untouched.
4. **Given** a platform that is not supported (for example 32-bit or FreeBSD), **When** the user runs `uv tool install jevpipe`, **Then** uv reports that no matching distribution exists and installs nothing; nothing tries to compile from source.
5. **Given** an installed binary, **When** the user runs `jevpipe filter` with a key set, **Then** it behaves exactly as a build from source of the same commit.

---

### User Story 2 - Release a version by pushing a tag (Priority: P1)

The maintainer bumps the version, pushes a tag such as `v0.1.0`, and the release pipeline does the rest:
it builds the package for all six platforms, installs and runs each on its platform, and uploads them to
TestPyPI as a trial run. Then it waits for the maintainer's approval in GitHub; after one click the
version goes to PyPI and a GitHub Release with notes and the packages is created. No token is stored anywhere; PyPI and TestPyPI
trust only this repository's release workflow.

**Why this priority**: It is how User Story 1 comes to exist, now and for every later version, and it
is where the technical risk of this milestone sits (every platform must build, including the keychain
and TLS dependencies).

**Independent Test**: Push the tag `v0.1.0`; the pipeline builds and checks six packages and puts
them on TestPyPI; after approval they are on PyPI and a GitHub Release exists. If anything fails
before the approval, nothing is on PyPI and the tag can be set again after the fix.

**Acceptance Scenarios**:

1. **Given** a tag `vX.Y.Z` whose version matches the one in `Cargo.toml`, **When** it is pushed, **Then** the pipeline builds one package per platform (Linux, macOS and Windows on x64 and arm64), installs each on its own platform and runs it, and uploads them to TestPyPI.
2. **Given** a tag whose version does not match `Cargo.toml`, **When** it is pushed, **Then** the pipeline fails before building and nothing is published.
3. **Given** one platform fails to build or its check fails, **When** the pipeline runs, **Then** nothing is uploaded for that tag.
4. **Given** the TestPyPI upload succeeded, **When** the maintainer approves the PyPI step in GitHub, **Then** the packages are uploaded to PyPI through trusted publishing with no stored token and carry PyPI attestations; without approval nothing reaches PyPI.
5. **Given** the PyPI upload, **When** it finishes, **Then** a GitHub Release named after the tag is created with generated notes and the packages attached.
6. **Given** a tag that is not of the form `vMAJOR.MINOR.PATCH` (for example `v0.1` or `test`), **When** it is pushed, **Then** no release runs.
7. **Given** a push or pull request without a tag, **When** CI runs, **Then** it runs the same checks as today and nothing is released.

---

### User Story 3 - A coding agent learns when and how to use jevpipe (Priority: P2)

A developer installs the skill with `npx skills add fabianboth/jevpipe --skill jevpipe` (or `gh skill install`). From
then on, when their coding agent faces many small judgments (which of 300 files deal with retries,
which log lines are real errors, triage of a list of issues), it reaches for `jevpipe filter` or
`jevpipe map` with a spend limit, a sensible threshold and a well-phrased question, reads only the
outcome, and acts on it itself. When the task is an exact text search, arithmetic or needs generated
text, it does not use jevpipe. When the binary or the API key is missing, it tells the user what to do
instead of trying to fix it itself.

**Why this priority**: The skill is how jevpipe reaches its main user, the coding agent. It follows
User Stories 1 and 2 because it is only useful with an installed binary.

**Independent Test**: Install the skill from the public repository with `npx skills add` into a project
and check that only the skill folder arrives. Then, in a coding agent with the skill and a key, ask
"which files in this repository deal with retrying failed requests?" and check that the agent uses
`jevpipe filter` with `--max-cost` and acts on the result; ask "find all calls of
`retry_with_backoff`" and check that it uses a text search instead.

**Acceptance Scenarios**:

1. **Given** the public repository, **When** a user runs `npx skills add fabianboth/jevpipe --skill jevpipe` or `gh skill install fabianboth/jevpipe jevpipe`, **Then** only the skill folder is installed (a few kilobytes, no source code, no binary), and the agent lists the skill as `jevpipe`.
2. **Given** a task with many independent judgments over records, files or lines, **When** the agent has the skill, **Then** it uses `filter` for one yes/no question and `map` for several questions or choice and score answers, sets `--max-cost`, and reads the summary on standard error.
3. **Given** a task that an exact text search, a count or a calculation answers, or one that needs generated text, **When** the agent has the skill, **Then** the skill tells it not to use jevpipe.
4. **Given** `jevpipe` is not on the path, **When** the agent would use it, **Then** the skill tells it to stop and tell the user that jevpipe needs installing, with the link to the README; it does not try to build or install it.
5. **Given** no API key is available (the run fails with the message naming both ways to provide one), **When** the agent meets it, **Then** the skill tells it to ask the user to run `jevpipe auth set-key` themselves or to set `OPENROUTER_API_KEY`; the agent never asks for, reads, prints or passes the key.
6. **Given** a run that ends with exit status 3, **When** the agent reads standard error, **Then** the skill has taught it that a limit stopped the run, which line to resume from, and not to rerun blindly with a higher limit.
7. **Given** records in the uncertain middle band of a probability, **When** the agent has the skill, **Then** it treats them as its own review items (it looks at them itself) rather than trusting them or handing them to the user.

---

### User Story 4 - A newcomer understands jevpipe from the README (Priority: P2)

Someone lands on the public repository or the PyPI page. Within a minute of reading they know what
jevpipe does, that it spends their OpenRouter credit, and that it is an unofficial tool built on
TypeSafe's Jev. Within five minutes they have installed it, stored a key and seen a first `filter` run
on their own files.

**Why this priority**: A public repository without a README, a license and an honest statement of who
is behind it is not usable, and not safe to rely on.

**Independent Test**: Give the README to someone who has not seen jevpipe; they install it, store a key
and run the first example without other help.

**Acceptance Scenarios**:

1. **Given** the README, **When** a reader looks at it, **Then** it holds, in this order: a one-sentence description, a short example of what a run looks like, installation (`uv tool install jevpipe`, the pipx alternative, and the one-line uv install for those without uv), the API key (`jevpipe auth set-key` or `OPENROUTER_API_KEY`), two or three examples, installing the agent skill, a note that runs spend OpenRouter credit with a pointer to `--max-cost`, the statement that jevpipe is unofficial and not affiliated with or endorsed by TypeSafe AI, and the license.
2. **Given** each command shown in the README, **When** it is run against the released binary (with a key), **Then** it works as shown.
3. **Given** the repository, **When** someone checks its license, **Then** a LICENSE file holds the Apache License 2.0 and the package metadata names `Apache-2.0`.

---

### User Story 5 - The repository is safe to make public (Priority: P3)

Before the switch to public, personal setup that does not belong to the project leaves the shared
files, and after the switch the repository is protected against leaked secrets and unreviewed changes
to `main`.

**Why this priority**: It is small and mostly one-time, but going public cannot be undone for anything
already pushed.

**Independent Test**: Clone the public repository on a macOS or Linux machine and open it in Claude
Code and VS Code: no hook fails, the editor shows its normal colors. In the repository settings, secret
scanning with push protection is on and `main` requires a pull request with passing CI.

**Acceptance Scenarios**:

1. **Given** the shared `.claude/settings.json`, **When** a session ends or asks for permission on any operating system, **Then** no notify hook runs from the repository; the maintainer's own notifications keep working from their own gitignored `.claude/settings.local.json`.
2. **Given** the shared `.claude/settings.json`, **When** it is compared with before, **Then** its permissions (allow and deny) are unchanged.
3. **Given** `.vscode/settings.json`, **When** the repository is opened in VS Code, **Then** the clippy check setting applies and no custom workbench colors do.
4. **Given** the history of every branch and tag that will be public, **When** it is scanned for secrets, **Then** none is found.
5. **Given** the public repository, **When** someone pushes a commit containing a known secret format, **Then** GitHub blocks the push; **When** someone tries to push to `main` directly or merge a pull request with failing CI, **Then** it is refused.

---

### Edge Cases

- **No uv on the machine**: the README's install section starts with uv's own one-line installer, then `uv tool install jevpipe`.
- **No Python on the machine**: uv needs an interpreter for its tool environment, even though jevpipe never uses it; uv downloads one once. The installed `jevpipe` command is still the bare binary.
- **uv's tool directory not on the path yet**: uv puts the command into `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows) and warns when that directory is not on the path; `uv tool update-shell` adds it, and the README names that command.
- **A `jevpipe` already in `~/.local/bin`** (for example a manual copy): uv refuses to overwrite it without `--force` and says so.
- **Downloaded package does not match its hash**: uv and pipx refuse to install it.
- **PyPI upload never approved**: PyPI keeps serving the previous version, and no GitHub Release is created for the tag.
- **Skill installed, binary older than the skill**: the skill names the jevpipe version it was written for and tells the agent to check `jevpipe <command> --help` when a flag is rejected.
- **The agent is in a non-interactive shell**: the skill never tells it to run `auth set-key` itself, since that reads the key from the user.
- **Headless Linux without a keychain**: the README says to use `OPENROUTER_API_KEY` there.
- **Binaries not code-signed**: macOS and Windows warn about unsigned binaries only when they carry a browser's download marker; uv does not set it. The PyPI attestations prove where a package was built, not a platform signature.

## Requirements *(mandatory)*

### Functional Requirements

**Release**

- **FR-001**: Pushing a tag `vMAJOR.MINOR.PATCH` (numbers only, no suffix) MUST build the PyPI package `jevpipe` of that version for Linux, macOS and Windows on both x64 and arm64: binary only, one package per platform, no source package.
- **FR-002**: The release MUST fail before uploading anything when the tag's version differs from the package version, or when any build or any check before the upload fails.
- **FR-003**: The Linux package MUST run on mainstream distributions, glibc- and musl-based, without installing any other package; storing the key in the keychain keeps working where a Secret Service is running.
- **FR-004**: The release MUST install and run each package on its own platform, upload all packages to TestPyPI, and upload them to PyPI only after the maintainer's approval in GitHub. Both uploads use trusted publishing; no token MUST be stored in the repository or its settings. The PyPI packages MUST carry PyPI attestations.
- **FR-004a**: After the PyPI upload, the release MUST create a GitHub Release named after the tag, with generated notes and the packages attached.
- **FR-005**: Only tags of the form `vMAJOR.MINOR.PATCH` MUST start a release, and only such tags MUST be allowed to publish to TestPyPI and PyPI.
- **FR-006**: Pushes and pull requests without a release tag MUST run the existing checks unchanged; `./check.ps1 -Fix` MUST keep passing.

**Installation**

- **FR-008**: `uv tool install jevpipe` and `pipx install jevpipe` MUST install the native binary of the release for the user's platform as the `jevpipe` command, with no Python launcher in between; `uv tool upgrade jevpipe` MUST move to a newer release.
- **FR-009**: On an unsupported platform no package MUST match, so nothing is installed and nothing tries to compile from source.
- **FR-010**: `jevpipe --version` MUST print the released version.
- **FR-010a**: Installing MUST be checked by CI, not by hand, once per release: each package is installed from its file with uv on its own platform, and the installed binary prints its version and, without a key, fails with exit status 2 and a message naming `OPENROUTER_API_KEY` (which runs the platform's keychain lookup), before anything is uploaded. The same files go to TestPyPI and PyPI. The maintainer's only manual step in a release is the approval.

**Agent skill**

- **FR-011**: The skill MUST live in `skills/jevpipe/` with a `SKILL.md` that follows the Agent Skills specification (a name matching the folder, a description that says what it does and when to use it, instructions in the body), so that `npx skills add fabianboth/jevpipe` and `gh skill install fabianboth/jevpipe jevpipe` install it; the repository MUST NOT have a `SKILL.md` at its root.
- **FR-012**: The skill folder MUST NOT contain a binary, install instructions or anything that runs on install; it MUST contain only text.
- **FR-013**: The skill MUST teach when to use jevpipe (many independent judgments over records, files or lines; enumerable step loops) and when not to (exact text search, counting and arithmetic, anything that needs generated text), and that jevpipe only decides while the agent acts.
- **FR-014**: The skill MUST teach how to choose between `filter` and `map`, how to phrase a question so it separates well (about the record itself, specific, avoiding questions that also match documents about the topic), how to choose a threshold (0.5 when both mistakes cost the same, higher when acting on a false yes is expensive, a middle band for review), and to pin a model version for reproducible runs.
- **FR-014a**: The skill MUST show a few typical `jq` usages to select and project `map` output; it adds no fallback and no install steps for `jq`, since an agent without it notices on the first try and has its own ways to read JSON. It MUST show how to produce the input from existing tools (`git ls-files`, `rg --files` or `find` for `--read-files`). The README names `jq` as recommended for `map`, with its install link, and no other prerequisite.
- **FR-015**: The skill MUST teach to set `--max-cost` (and `--max-time` where a deadline matters) for every run over more than a handful of records, the meaning of exit statuses 0, 1, 2 and 3, how to resume after exit status 3, and to read the one-line summary on standard error.
- **FR-016**: The skill MUST tell the agent, when `jevpipe` is missing, to tell the user it needs installing with the link to the README, and, when no key is available, to ask the user to run `jevpipe auth set-key` or set `OPENROUTER_API_KEY` themselves; it MUST forbid the agent from asking for, reading, printing or passing the API key.
- **FR-017**: The skill MUST tell the agent to treat records in the uncertain band as its own review items.
- **FR-017a**: The skill MUST be written with the guidance of the `skill-creator` skill (structure, a description that triggers on the right tasks, progressive disclosure into `references/` where the body grows long); its evaluation loop is 005.
- **FR-018**: The skill MUST state the jevpipe version it was written for, and the lessons in `specs/manual/skill-learnings.md` that it relies on MUST be presented as current guidance to be revisited in 005, not as measured facts.
- **FR-019**: Every flag named in `SKILL.md` and in the README MUST exist in the binary's help, the skill's name MUST equal its folder name, and the README MUST have no relative links; an automated test checks this against the real binary without network access.

**README and license**

- **FR-020**: The repository MUST have a README with the content and order of User Story 4, scenario 1, short enough to read in a couple of minutes: no section longer than a screen, details left to `--help`.
- **FR-020a**: The README's shape MUST be taken from studying the READMEs of a few well-regarded command-line tools (for example ripgrep, fd, uv, jq and Simon Willison's `llm`): what they show first, how they present install and a first example, and what they leave out. The first screen MUST show what jevpipe does and a real example of input and output, before any explanation; plain language, no jargon without an example, no badge wall.
- **FR-020b**: The README MUST also be the PyPI project description, so every link and image in it MUST be absolute and it MUST use only Markdown that PyPI renders; there is no second README.
- **FR-021**: The repository MUST have a LICENSE file with the Apache License 2.0, and the package metadata MUST name `Apache-2.0`.
- **FR-022**: The README MUST state that jevpipe is unofficial and not affiliated with or endorsed by TypeSafe AI, and that Jev is TypeSafe AI's model.

**Going public**

- **FR-023**: The shared `.claude/settings.json` MUST NOT contain the notify hooks, and the repository MUST NOT contain the notify script; the permissions MUST stay as they are. The hooks and the script move to the maintainer's user-level Claude Code settings.
- **FR-024**: `.vscode/settings.json` MUST keep only the clippy check setting.
- **FR-025**: Before the switch, the history of every branch and tag that becomes public MUST be scanned for secrets, with no finding. (Done 2026-09-27.)
- **FR-026**: After the switch, secret scanning with push protection MUST be on (done 2026-09-27), and `main` MUST accept changes only through pull requests whose CI passed.
- **FR-027**: The first release is 0.1.0 itself, without a separate release candidate: everything before the approval publishes nothing to PyPI, so a run that fails before the TestPyPI upload is fixed and the tag set again, and one that fails after it is fixed in the next patch version.

### Key Entities

- **Release**: one version of jevpipe, identified by its tag; six platform packages on PyPI (with attestations) and a GitHub Release with notes and the same packages.
- **Platform**: an operating system and processor pair: Linux, macOS or Windows on x64 or arm64.
- **Agent skill**: the `skills/jevpipe/` folder; installed into an agent's skill directory independently of the binary and versioned with the repository.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a fresh machine of each of the six platforms with uv, one command from the README leads to a working `jevpipe --version` in under one minute; without uv, two commands.
- **SC-002**: From pushing a release tag to checked packages on TestPyPI takes under 30 minutes with no manual step; PyPI and the GitHub Release follow one approval.
- **SC-002b**: No version reaches PyPI whose packages were not installed and run on all six platforms.
- **SC-002a**: The installed command starts as fast as the binary itself: no language runtime starts before it.
- **SC-003**: A person who has never seen jevpipe goes from the README to the result of a first `filter` run on their own files in under five minutes.
- **SC-003a**: The README is read top to bottom in under three minutes, and a reader who stops after the first screen can say in one sentence what jevpipe does.
- **SC-004**: Installing the skill copies under 50 kilobytes into the agent's skill directory.
- **SC-005**: In the two tasks of User Story 3's independent test, an agent with the skill uses jevpipe with a spend limit for the conceptual question and a text search for the exact one.
- **SC-006**: 100% of the flags named in the skill and the README exist in the released binary.
- **SC-007**: No secret is found in any commit of the public repository, and GitHub reports no secret-scanning alert after the switch.
- **SC-008**: The full check (`./check.ps1`) passes on Linux, Windows and macOS without network access, as before.

## Assumptions

- The packages are binary-only wheels (maturin's `bin` mode), the way ruff and uv ship: the wheel holds the native executable, which uv and pipx put on the path as is (verified: same file, 28 ms start like the bare binary). The Linux wheels are static musl builds that serve glibc and musl distributions alike.
- The name `jevpipe` is free on PyPI and TestPyPI; pending trusted publishers for `fabianboth/jevpipe` take it at the first upload (they do not reserve it before). TestPyPI is the staging step, as in the maintainer's earlier PyPI projects; the approval is a GitHub environment with the maintainer as required reviewer.
- npm is not a channel: a Rust binary on npm always starts through a Node.js launcher. The `jevpipe` name on npm stays reserved with the 0.0.0 placeholder by the npm account `bothlabs`.
- Standalone install scripts are not a channel in 0.1.0; cargo-dist can add them later as a second channel for users without uv.
- The Linux keychain uses a pure-Rust Secret Service client and needs no system library; the TLS stack is the remaining build risk for some platforms, which the first release run exposes before anything is uploaded.
- uv installs tools into `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows), the per-user directory many Linux distributions already have on the path; no administrator rights are needed.
- Binaries are not code-signed or notarized in 0.1.0; uv does not mark downloads as coming from the internet, so macOS and Windows do not block them. PyPI's attestations are the trust signal instead.
- The skill is written from the current CLI, the TypeSafe documentation and `specs/manual/skill-learnings.md`; its trigger accuracy, thresholds and phrasing advice are measured and revised in 005.
- "Unofficial, not affiliated" wording is the standard for third-party tools built on another company's product; no TypeSafe brand guidelines forbidding the name were found.
- The commit author email in the history becomes public with the repository; this is accepted.
- The repository keeps its AI tooling (`CLAUDE.md`, specs, spec-kit, the review skill); they document how the project is built.

## Out of Scope

- The calibration study, evaluations of the skill and the example pipelines (005).
- Standalone install scripts and bare binary downloads (cargo-dist), npm, Homebrew, crates.io, winget, Scoop and other package managers.
- Code signing and notarization.
- Automatic updates or an update command.
- Embedding the skill in the binary (a `jevpipe skill install` command).
- Contribution guidelines, a security policy and a code of conduct.

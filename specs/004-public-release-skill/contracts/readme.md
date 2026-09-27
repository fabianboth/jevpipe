# Contract: README

`README.md` at the repository root, also the PyPI description; a landing page, under ~120 lines,
three badges (CI, PyPI version, license), no table of contents. Commands in fenced `sh`/`powershell`
blocks without `$` prompts; output in separate blocks. Every link and image URL absolute
(`https://github.com/fabianboth/jevpipe/...`), no GitHub alerts, no `<picture>`: PyPI renders
relative links broken and alerts as literal text. Shape from research R8.

## Sections, in this order

1. **`# jevpipe`** and the one sentence from `Cargo.toml`: "A Unix pipe for typed decisions: stream
   records in, get calibrated decisions out."
2. **A real run** (first screen): one command over this repository, e.g.
   `git ls-files | jevpipe filter "Does this file parse command line arguments?" --read-files`, its
   real output and summary line, and one sentence: like grep, but the match is a question, answered
   by TypeSafe's Jev model through OpenRouter.
3. **Install**: `uv tool install jevpipe`; then one line each: `pipx install jevpipe` as the
   alternative, uv's own one-line installer for those without uv (macOS/Linux and Windows), and
   `uv tool update-shell` if `jevpipe` is not found afterwards; upgrading with
   `uv tool upgrade jevpipe`.
4. **API key**: `jevpipe auth set-key` (stored in the system keychain); or `OPENROUTER_API_KEY`,
   which takes precedence and is the way on headless Linux.
5. **Examples** (two or three one-liners): a log `filter` piped to `head`; a `map` with typed
   questions and `jq`, with one output line; a line pointing to `jevpipe filter --help` and
   `jevpipe map --help` for exit statuses, question types and all flags; `jq` recommended for
   `map`, with its install link.
6. **Use it from a coding agent**: `npx skills add fabianboth/jevpipe --skill jevpipe` (or
   `gh skill install fabianboth/jevpipe jevpipe`); one sentence on what the skill teaches.
7. **Cost**: every record is a request billed to your OpenRouter credit; cap a run with
   `--max-cost 0.50` (and `--max-time 10m`); a stopped run exits 3 and names the line to resume from.
8. **Unofficial**: jevpipe is an independent project, not affiliated with or endorsed by TypeSafe AI;
   Jev is TypeSafe AI's model.
9. **License**: Apache-2.0. One line under it: building from source needs Rust (rustup picks the
   pinned toolchain) and PowerShell 7 for `./check.ps1`.

## Checked automatically (`tests/cli/docs.rs`)

Every `--flag` in the README exists in the binary's help, and the README
contains no relative Markdown links or images.

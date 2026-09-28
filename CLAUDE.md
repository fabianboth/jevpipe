# Instructions

jevpipe is a Unix pipe for typed decisions. Records stream in (plain lines, JSONL lines or file
paths), calibrated decisions stream out: `filter` prints the kept lines unchanged, `map` prints one JSON
line per record with its answers. The decisions come from Jev
(TypeSafe's System One model), through OpenRouter or TypeSafe's own API (the `provider` config key),
or any server speaking the same API. A coding agent writes the loop,
jevpipe makes the many small yes/no, pick-one or score judgments inside it, and only the outcome reaches
the agent's context. The draft idea lives in `specs/manual/idea-draft.md`.

Shape: a single Rust binary with the subcommands `filter` and `map` (plus `config` for the user's
defaults and `auth` for the stored API key), and an agent skill (`skills/jevpipe/SKILL.md`) that
teaches coding agents when to reach for it. Releases ship as binary-only wheels on PyPI, installed
with `uv tool install jevpipe`.

## Project Rules
- Decide, don't act: no planning, no text generation, no executing actions. The calling script owns that.
- Stateless: a multi-turn driver sends the current state each step; jevpipe keeps no history.
- Take the wire format from the official TypeSafe API docs, never from guesses or old examples.
- Lint levels live in the `[lints]` table of `Cargo.toml` (thresholds in `clippy.toml`); change one
  there, never with `#[allow]` or `#[expect]` in code. Exception: `#![expect(clippy::unwrap_used)]`
  at an integration test crate root (rust-clippy#13981).
- `unsafe` is forbidden.
- The API key comes from the provider's variable (`OPENROUTER_API_KEY` or `TYPESAFE_API_KEY`) or the
  OS keychain, never from a file in the repo.
  Tests never touch the network, the real keychain or the user's config: binary tests set
  `JEVPIPE_CONFIG` to a temp file, whose `base-url` points at the local stand-in.

## Layout
`src/` for product code (`main.rs` only wires the CLI; the logic lives in the library crate `lib.rs`),
`tests/` for integration tests, `skills/jevpipe/` for the agent skill, `examples/` for the example
pipelines, `demo/` for the README's terminal recording (a [VHS](https://github.com/charmbracelet/vhs)
tape, re-recorded by hand from the repo root with `vhs demo/demo.tape`; its bash needs jevpipe, jq,
python3 and `TYPESAFE_API_KEY`). `bench/` is the code search benchmark: its own uv project (Python,
ruff, pyright strict, pytest), run by hand stage by stage (`uv run bench <stage>`), results committed in `bench/results/`,
downloads in the gitignored `bench/.cache/`; the product never depends on it.

## Code Style
- NO COMMENTS. We strive for self-explanatory code style. Needing one normally means the code is not good enough (names, functions, extraction) — improve the code instead. The exception is a fact the code *cannot* state like an external API's behaviour. Doc comments on clap types are not comments: they are the `--help` text.
- Reusable code via functions and types (+ reuse existing code); prefer a proven crate over hand-rolling
- Max 3 parameters per function besides `self` (clippy enforces 4 counting `self`): extract the type they imply or split the function; a parameter struct only as a last resort
- One module per concept
- `Option` only where absence carries information (not bound yet, genuinely optional, transient cache)
- Exhaustive `match`: list the variants (`A | B => Err(...)`), never `_` on an enum, so a new variant fails to compile until every `match` handles it; for a foreign `#[non_exhaustive]` enum use `==` or `matches!`
- No `unwrap`, `expect` or `panic!` in product code: return an error with context
- Private by default; `pub` only where another crate needs it (the binary or the integration tests)
- Tests use the real product (the real binary against local stand-ins, never a code-built copy) and assert behaviour, not authored values; only what the binary cannot be pointed away from (the OS keychain) is tested in-process against a stand-in

## Way of working
- Your knowledge about Rust crates, the TypeSafe API, Jev and OpenRouter is potentially incomplete or outdated. Research in the web (and don't fall for old resources)
- If you don't know how things work, and it is cheap to test, you quickly prototype it to test your assumptions (web search prefered first)
- A script longer than a few lines, or containing `\\`, goes to a file via Write and runs from there: Bash commands are cut at ~8 KB and turn `\\` into `\` (claude-code#93915).

## Verification
- Tooling: `check.ps1` needs PowerShell 7.3+ (`pwsh`); rustup installs the toolchain pinned in
  `rust-toolchain.toml` on first use; `cargo-deny` is
  the one extra tool (a prebuilt binary from its GitHub releases into `~/.cargo/bin`).
- `./check.ps1 -Fix` must pass. Stages: 1) **format** - `cargo fmt` (verifies, or fixes with `-Fix`)
  2) **lint** - `cargo clippy --all-targets` with warnings as errors: types, lints and style all fail
  this one command 3) **test** - `cargo test` 4) **deny** - `cargo deny check` (advisories, licenses,
  duplicate crates, sources; the rules live in `deny.toml`).
- Without `-Fix` every stage runs `--locked`: a `Cargo.lock` that is out of date fails.
- `-Filter <text>` narrows the test stage to tests whose name contains the text, to iterate fast
- `-Stage <names>` runs only those stages (`-Stage test`, `-Stage format,lint`).
- CI (`.github/workflows/ci.yml`) runs the same script and is the authority.
- `bench/check.ps1 [-Fix]` checks the benchmark offline (ruff format, ruff check, pyright strict,
  pytest; needs ripgrep on the path). `.github/workflows/bench.yml` runs it only when `bench/`
  changes, outside the required gate. Its rule levels live in `bench/pyproject.toml`, never in code.

## Releases
- A tag `vMAJOR.MINOR.PATCH` equal to the `Cargo.toml` version runs `.github/workflows/release.yml`:
  maturin builds six wheels, each is installed and run on its own platform, then TestPyPI, the
  maintainer's approval (environment `pypi`), PyPI with attestations, and a GitHub Release.
- `pyproject.toml` holds the PyPI metadata; the version comes from `Cargo.toml`.
- `README.md` is also the PyPI description, so every link and image in it is absolute.

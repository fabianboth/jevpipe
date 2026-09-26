# Instructions

jevpipe is a Unix pipe for typed decisions. Records stream in (plain lines or JSONL),
calibrated decisions stream out as JSONL that keeps each record's id. The decisions come from Jev
(TypeSafe's System One model) or any server speaking the same API. A coding agent writes the loop,
jevpipe makes the many small yes/no, pick-one or score judgments inside it, and only the outcome reaches
the agent's context. The draft idea lives in `specs/manual/idea-draft.md`.

Planned shape: a single Rust binary with the subcommands `filter`, `map` and `serve`, plus an agent skill
(`skills/jevpipe/SKILL.md`) that teaches coding agents when to reach for it. First milestone: `filter`
end to end on files and stdin, with offline tests.

## Project Rules
- Decide, don't act: no planning, no text generation, no executing actions. The calling script owns that.
- Stateless: a multi-turn driver sends the current state each step; jevpipe keeps no history.
- Take the wire format from the official TypeSafe API docs, never from guesses or old examples.
- Lint levels live in the `[lints]` table of `Cargo.toml` (thresholds in `clippy.toml`); change one
  there, never with `#[allow]` or `#[expect]` in code.
- `unsafe` is forbidden.
- API keys come from the environment, never from a file in the repo. Tests never touch the network.

## Layout
`src/` for product code (`main.rs` only wires the CLI; the logic lives in the library crate `lib.rs`),
`tests/` for integration tests, `skills/jevpipe/` for the agent skill, `examples/` for the example
pipelines.

## Code Style
- NO COMMENTS. We strive for self-explanatory code style. Needing one normally means the code is not good enough (names, functions, extraction) — improve the code instead. The exception is a fact the code *cannot* state like an external API's behaviour.
- Reusable code via functions and types (+ reuse existing code); prefer a proven crate over hand-rolling
- Max 3 parameters per function besides `self` (clippy enforces 4 counting `self`): extract the type they imply or split the function; a parameter struct only as a last resort
- One module per concept
- `Option` only where absence carries information (not bound yet, genuinely optional, transient cache)
- Exhaustive `match`: list the variants (`A | B => Err(...)`), never `_` on an enum, so a new variant fails to compile until every `match` handles it; for a foreign `#[non_exhaustive]` enum use `==` or `matches!`
- No `unwrap`, `expect` or `panic!` in product code: return an error with context
- Private by default; `pub` only where another crate needs it (the binary or the integration tests)
- Tests use the real product (the real binary against a local stub of the API, never a code-built copy) and assert behaviour, not authored values

## Way of working
- Your knowledge about Rust crates, the TypeSafe API, Jev and OpenRouter is potentially incomplete or outdated. Research in the web (and don't fall for old resources)
- If you don't know how things work, and it is cheap to test, you quickly prototype it to test your assumptions (web search prefered first)
- A script longer than a few lines, or containing `\\`, goes to a file via Write and runs from there: Bash commands are cut at ~8 KB and turn `\\` into `\` (claude-code#93915).

## Verification
- Tooling: rustup installs the toolchain pinned in `rust-toolchain.toml` on first use; `cargo-deny` is
  the one extra tool (a prebuilt binary from its GitHub releases into `~/.cargo/bin`).
- `./check.ps1 -Fix` must pass. Stages: 1) **format** - `cargo fmt` (verifies, or fixes with `-Fix`)
  2) **lint** - `cargo clippy --all-targets` with warnings as errors: types, lints and style all fail
  this one command 3) **test** - `cargo test` 4) **deny** - `cargo deny check` (advisories, licenses,
  duplicate crates, sources; the rules live in `deny.toml`).
- Without `-Fix` every stage runs `--locked`: a `Cargo.lock` that is out of date fails.
- `-Filter <text>` narrows the test stage to tests whose name contains the text, to iterate fast
- CI runs the same script on `ubuntu-latest`, `windows-latest` and `macos-latest` and is the authority.

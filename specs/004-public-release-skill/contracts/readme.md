# Contract: README

`README.md` at the repository root, also the PyPI description. A landing page for people whose
coding agent should use jevpipe: little prose, real outputs, alternatives collapsed. Every link and
image URL absolute (`https://github.com/fabianboth/jevpipe/...`; section links as absolute GitHub
URLs), no GitHub alerts; `<details>` and centered HTML are allowed (PyPI renders both). Worked out
with the maintainer after the first draft.

## Sections, in this order

1. **Hero** (centered, no code): name, "Give your coding agent a System 1.", one subtitle line, three
   badges (CI, PyPI, license), links to Quick start · Example · Use cases · Cost.
2. **Quick start**: three commented lines (`uv tool install jevpipe`, `jevpipe auth set-key`,
   `npx skills add fabianboth/jevpipe --skill jevpipe`); one sentence on the OpenRouter key and
   `OPENROUTER_API_KEY` where there is no keychain, and a link to install uv; collapsed: pipx and
   `gh skill install`.
3. **Example**: one lead sentence, the real `filter` run over ripgrep 15.1's commits with its output
   (comments mark the matches and the summary); collapsed: the `map` release-notes example with one
   real JSON line.
4. **Use cases**: "each item is judged on its own"; a table of job, question per item and answer
   type.
5. **Commands**: a two-row table (`filter`, `map`); input like grep, with three commented examples
   (piped lines, lines of a file, `--read-files`); pointer to `--help`.
6. **Cost and speed**: a table of measured runs by input size, cost per 1,000 records; `--max-cost`
   and exit status 3; `jevpipe config set max-cost` for a default cap.
7. **License**: Apache-2.0, not affiliated with TypeSafe AI, Jev credited, building from source.

## Checked automatically (`tests/cli/docs.rs`)

Every jevpipe flag in the README exists in the binary's help, and every Markdown, reference or HTML
link and image target is an absolute `https://` URL.

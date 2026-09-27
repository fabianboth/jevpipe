# Feature Specification: Map Command

**Feature Branch**: `002-map-command`
**Created**: 2026-09-27
**Status**: Draft
**Input**: User description: "jevpipe v2 (002-map): add the `map` command and reshape `filter`, completing the CLI command suite. No `serve` command. See conversation for all decisions."

## Context

jevpipe puts typed judgments inside a Unix pipeline: records stream in, Jev answers questions about
each one, and only the result comes out. The first milestone delivered `filter` ("which records?").
This milestone completes the command suite with `map` ("what about each record?"): several typed
questions (yes/no, pick one, score) asked about every record in one call, one JSON line of answers per
record out.

The two commands differ in what they print, and that is the whole split:

- `filter` prints the kept records themselves, unchanged, like `grep`. It needs no other tool and feeds
  `xargs`, `head` or `wc` directly.
- `map` prints one JSON line per record: the record and the service's answers. A calling script selects
  and projects with `jq`, so only the fields it needs reach an agent's context.

When both a selection and labels are needed, the yes/no question goes into `map`'s questions and `jq`
selects afterwards. Every record is then sent once, which is cheaper and faster than chaining `filter`
into `map`.

The interactive step loop that the idea draft called `serve` (for example a browser driven by a script,
one decision per step) needs no command of its own: the driver runs one `map` per step with that step's
questions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ask several typed questions about every record (Priority: P1)

A developer or agent writes the questions once into a small JSON file, in the service's documented
question format, and pipes records through `jevpipe map -f questions.json` (or passes the same JSON
inline with `-q`). Each record is sent once with
all questions; each output line holds the record and the answers, ready for `jq`.

**Why this priority**: It is the new capability of this milestone: labelling, triage and scoring of
records, with all questions answered in one call per record.

**Independent Test**: Pipe three lines through `map` with a questions file holding a yes/no, a choice
and a score question, against a stand-in service with known answers; the output is three JSON lines in
input order, each with the line and exactly the stand-in's answers, and the stand-in received one
request per line carrying all three questions.

**Acceptance Scenarios**:

1. **Given** a questions file with a yes/no, a choice and a score question and three lines on standard input, **When** the user runs `jevpipe map -f questions.json`, **Then** exactly one request per line is sent carrying all three questions, and standard output holds three JSON lines in input order, each `{"record": "<the line>", "answers": {...}}`.
2. **Given** an answered record, **Then** its `answers` hold every question by the name used in the questions file, exactly as the service returned them (for example the chosen option with its probabilities and confidence, or the score with its levels).
3. **Given** a JSONL line such as `{"test":"login_timeout"}`, **When** it is answered, **Then** it is sent as text and appears in `record` as that text, so `jq '.record | fromjson | .test'` recovers its fields.
4. **Given** two files named after the questions, **When** the user runs `jevpipe map -f questions.json a.txt b.txt`, **Then** the records of `a.txt` are answered and printed before those of `b.txt`.
5. **Given** every record answered, **When** the run finishes, **Then** the exit status is 0 and one summary line (records, answered, skipped, failed, cost, duration) is written to standard error.
6. **Given** a record that still fails after all retries, **When** the run finishes, **Then** it has its own output line with `"outcome": "failed"` and a short reason instead of answers, all other records are answered, standard error names its line number, and the exit status is 2.
7. **Given** a driver that writes one line, waits for the answer and only then writes the next, **When** each line arrives, **Then** its answer line is written without waiting for further input, so one `map` process can also serve a step-by-step loop.

---

### User Story 2 - Ask questions about files (Priority: P2)

A developer or agent pipes a list of file paths (for example from `git ls-files`) into
`jevpipe map -f questions.json --read-files`; each file's path and content are judged, and each output
line carries the path as its record.

**Why this priority**: It extends the flagship "semantic grep" to several complex questions per file,
with the path as the handle the agent acts on.

**Independent Test**: Create a few text files and one binary file, pipe their paths through
`map --read-files`; the text files are answered with their paths as records, the binary file has a
skipped line, and the run exits 0.

**Acceptance Scenarios**:

1. **Given** a list of paths, **When** the user runs `jevpipe map -f questions.json --read-files`, **Then** each file's path and content are judged, and each output line's `record` is the path as read.
2. **Given** a binary, non-UTF-8 or empty file or a directory, **When** it is processed, **Then** it gets an output line with `"outcome": "skipped"` and a reason, no service call is made, and the exit status is not affected.
3. **Given** a path that does not exist, **When** it is processed, **Then** it gets an output line with `"outcome": "failed"` and the reason "not found", and the exit status is 2.
4. **Given** a file larger than the service accepts, **When** it is processed, **Then** its content is cut to fit, it is still answered, and its output line carries `"truncated": true`.

---

### User Story 3 - A leaner `filter` (Priority: P3)

`filter` becomes the pure grep: it prints the kept lines unchanged and nothing else. Structured output
moves to `map`, so `filter` loses `--json` and `--all`. Failures on standard error name the line
number instead of the record, so a large record cannot flood an agent's context.

**Why this priority**: It keeps each command to one job and one output shape, and it keeps the log
small for both commands. `filter` itself already works.

**Independent Test**: Run `filter --json` and check it is rejected as a usage error; run `filter` over
input with a failing record and check standard error names the line number but not the record.

**Acceptance Scenarios**:

1. **Given** `filter` with `--json` or `--all`, **When** the user runs it, **Then** it is rejected as an unknown option, exit status 2.
2. **Given** input whose fourth line fails, **When** `filter` or `map` finishes, **Then** standard error holds `jevpipe: line 4: <reason>` and not the line's content.
3. **Given** input with blank lines, **When** a record fails, **Then** its line number counts the blank lines, so it matches the line in the input.
4. **Given** `--read-files` and a file cut to fit, **When** `filter` or `map` finishes, **Then** the summary line reports how many files were truncated.
5. **Given** everything else, **When** `filter` runs, **Then** it behaves as in the first milestone: kept lines byte-identical (a last line without a line terminator gets `\n`) and in input order, the same thresholds and exit statuses.

---

### Edge Cases

- **Neither or both of `-q` and `-f` given; questions file missing or unreadable; questions not JSON or not a non-empty object of questions; a question name used twice**: usage error with a message naming the problem (and the question, where one is at fault), before any input is read or request sent, exit status 2.
- **A question with an unknown type, without instructions, or without the criteria its type needs** (options for a choice, levels for a score), **or outside the documented limits** (more than 255 options, fewer than 2 or more than 10 levels): usage error as above. Anything else the service rejects stops the run at the first request with the service's message, as any run-level error does.
- **Question names** are free: whatever names the file uses are sent and come back in `answers`. The output's own fields (`record`, `answers`, `outcome`, `reason`, `truncated`) never collide with them because the answers are nested.
- **A response that lacks an answer for one of the questions, or carries one of the wrong type**: the run stops with an "unexpected answer" error and exit status 2, as `filter` does for an answer in an unexpected shape: the service is not behaving as documented, and every further record would meet the same problem.
- **Blank lines** are not records: nothing is sent and nothing is printed for them, but they count for line numbers.
- **Empty input** in `map`: no request, summary with zero records, exit status 0. (`filter` keeps exit status 1 for "nothing kept".)
- **A line over 100,000 characters** (the limit file content is cut to): it fails as "too large" without a request, in both commands; jevpipe reads at most 400,000 bytes of it and skips the rest, so memory stays bounded. Lines are not cut and sent like file content, because the line is the record itself. In `map`, its output line carries the line's first 100,000 characters.
- **A text line that is not valid UTF-8** in `map`: it fails; its output line carries the line with invalid bytes replaced, and standard error names its line number.
- **An input file named on the command line cannot be opened or is UTF-16**: reported on standard error by its file name, counted as failed, exit status 2; it contributes no lines, so it has no output line and line numbering continues with the next file. This applies to both commands.
- **Skipped records** in `map` get an output line but do not affect the exit status, as in `filter`.
- **The consumer of standard output goes away** (for example `head -3`): jevpipe stops sending requests and exits without an error, as in `filter`.
- **A record whose answer depends on a truncated file**: marked `"truncated": true` in `map`; counted in the summary for both commands.
- **Run-level service errors** (key rejected, no credits, unknown model, malformed request): the run stops at the first one with the service's message and exit status 2, as in `filter`.

## Requirements *(mandatory)*

### Functional Requirements

**Commands**

- **FR-001**: jevpipe MUST offer exactly two commands, `filter` and `map`.
- **FR-002**: `jevpipe map (-q <JSON> | -f <FILE>) [FILE...]` MUST take the questions inline from `-q`/`--questions` or from the file named by `-f`/`--questions-file` (exactly one of the two) and read the records from the named files in argument order, or from standard input when no file is named or a file is named `-`.
- **FR-003**: Both commands MUST accept `--read-files`, `--concurrency`, `--model` and `--request-timeout` with the meaning and defaults of the first milestone; `--threshold` MUST remain `filter`-only.
- **FR-004**: `filter` MUST no longer accept `--json` or `--all`.

**Questions**

- **FR-005**: The questions (inline or in the file) MUST be a JSON object of named questions in the service's documented format: each question has a `type` (`noul`, `choice` or `score`) and `instructions`; a choice has `criteria` mapping option names to descriptions; a score has `criteria` as a list of levels from low to high; a yes/no question may have `criteria`. They MUST be sent as the request's questions unchanged.
- **FR-006**: Before reading any input, `map` MUST check the questions' shape (valid JSON, a non-empty object, each question an object with a known type, instructions, and the criteria its type requires within the documented limits: 1 to 255 options for a choice, 2 to 10 levels for a score) and reject malformed questions as a usage error naming the problem.
- **FR-007**: Each record MUST be answered with a single request carrying all questions.

**Records**

- **FR-008**: Each non-blank line MUST be one record, sent to the service as text; JSONL lines are text like any other line. A line over 100,000 characters MUST fail as "too large" without a request. With `--read-files`, the line is a file path and the file's path and content are judged, with the first milestone's skip, fail and truncation rules.
- **FR-009**: Line numbers MUST count every input line, including blank lines, over all inputs in order, as if they were joined into one stream.

**`map` output**

- **FR-010**: `map` MUST write exactly one JSON line per record to standard output, in input order, each written as soon as it and every record before it are decided.
- **FR-011**: An answered record's line MUST be `{"record": <line>, "answers": <answers>}`, where `record` is the input line as a string (the path with `--read-files`) and `answers` are the service's answers exactly as returned; it MUST add `"truncated": true` when the judged file content was cut.
- **FR-012**: A skipped or failed record's line MUST be `{"record": <line>, "outcome": "skipped" | "failed", "reason": <short reason>}`.
- **FR-013**: `map`'s exit status MUST be 0 when no record failed (including empty input) and 2 when any record failed or the run stopped on an error; skipped records MUST NOT affect it.

**Standard error (both commands)**

- **FR-014**: Each failed record MUST be reported on standard error by its line number and reason only, never with the record's content; a failed input file MUST be reported by its file name.
- **FR-015**: Every run MUST end with exactly one summary line on standard error: records, the command's result count (kept for `filter`, answered for `map`), skipped, failed, truncated files when any, total cost reported by the service, and duration. The model is not shown: pin one with `--model` for reproducible runs.

**Unchanged from the first milestone**

- **FR-016**: Concurrency, retries, per-request timeout, run-level errors, configuration through `OPENROUTER_API_KEY` and `JEVPIPE_BASE_URL`, the default model and stopping when the output consumer goes away MUST work for `map` as they do for `filter`.
- **FR-017**: `jevpipe map --help` MUST describe the command, every option and the questions format with an example.

**Verification**

- **FR-018**: All behaviour in this spec MUST be verifiable by automated tests that run the real binary against a local stand-in for the service, without network access or an API key. The stand-in's answers for choice and score questions MUST follow real recorded responses.

### Key Entities

- **Record**: one non-blank input line (a file path with `--read-files`), with its line number in the whole input.
- **Questions**: the named, typed questions given with `-q` or `-f`, the same for every record in a run; sent unchanged.
- **Answers**: the service's answers for one record, one per question name, passed through as returned.
- **Outcome**: per record, one of answered (with answers), skipped (deliberately not judged) or failed (could not be decided), each with a short reason when not answered.
- **Run summary**: counts of records, results, skipped, failed and truncated, total cost and duration; determines the exit status.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a run with N non-blank records, `map` writes exactly N output lines, in input order, each valid JSON, in 100% of test scenarios.
- **SC-002**: Asking three questions about a record costs one request, not three.
- **SC-003**: 1,000 short records with three questions are answered in under 30 seconds under normal service conditions.
- **SC-004**: In a step loop that writes one record and waits, each answer is written within 1 second of the service's reply.
- **SC-005**: No record content ever appears on standard error, in 100% of test scenarios, including failures of records larger than 100,000 characters.
- **SC-006**: Malformed questions, inline or in a file, are rejected before any request is sent, in 100% of test scenarios.
- **SC-007**: The full automated test suite passes on Linux, Windows and macOS with no network access and no API key.
- **SC-008**: An agent that knows `jq` can write a triage pipeline (two questions, select one label) from `jevpipe map --help` alone.

## Assumptions

- Records are sent as text only: the first milestone's spike sent the same record as a JSON object and as its text and got the same probabilities (0.82–0.83) and nearly the same token count. Structured state can return if a measurement shows it helps.
- The output echoes the record, like `grep` prints the lines it matches: `map`'s output is meant for `jq` and the calling script, not for an agent to read whole, so selecting and projecting in the pipe keeps the agent's context small. A failed record keeps its line on standard output so a step-loop driver waiting for one answer per record never hangs, and a script can see which record got no answer.
- The questions are given inline (`-q`, handy for agents: one command, no file to write or clean up) or from a file (`-f`, handy for people and long question sets), as two options of which exactly one is required, like `grep -e` / `grep -f`. They follow the service's documented format verbatim (TypeSafe's System One `questions` object, checked against the official docs on 2026-09-27), so users learn one format and new question fields reach the service without a jevpipe change.
- A step loop runs one `map` per step. The extra connection setup per step (roughly 50–150 ms) is accepted; per-record questions for a long-running loop can follow if it matters.
- Removing `filter`'s `--json` and `--all` is a breaking change; the first milestone was not released, so no compatibility period is needed.

## Out of Scope

- A `serve` command, and per-record questions.
- Caching identical requests, and budgets on records, cost or run time.
- Other providers than OpenRouter, and shared key names across providers.
- Inverted matching, sorting or ranking by probability.
- Sending records as structured JSON state.
- The agent skill (`skills/jevpipe/SKILL.md`) and the example pipelines; they follow once the command suite is complete.

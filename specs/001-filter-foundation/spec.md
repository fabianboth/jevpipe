# Feature Specification: Filter Foundation

**Feature Branch**: `001-filter-foundation`
**Created**: 2026-09-26
**Status**: Draft
**Input**: User description: "jevpipe filter: first milestone, a clean foundation (best practice, lean, not overengineered). See conversation for all decisions."

## Context

jevpipe lets a script or coding agent put a typed judgment inside a Unix pipeline: records stream in,
a decision model answers one question per record, and only the result comes out. This first milestone
delivers `filter`: keep the records for which a yes/no question is answered "yes". It is also the
foundation `map` and `serve` build on later, so its input handling, service access, concurrency, error
handling and output conventions must be clean enough to reuse.

The decision service is Jev, reached through OpenRouter. Its behaviour was measured before writing this
spec; see [api-spike.md](api-spike.md).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Filter lines by a yes/no question (Priority: P1)

A developer or agent pipes plain text lines into `jevpipe filter "<question>"` (or names files to read
lines from) and gets back exactly the lines for which the answer is "yes", unchanged and in their
original order, ready for the next tool in the pipe.

**Why this priority**: This is the core of the product. Everything else refines what a record is or how
much detail comes out.

**Independent Test**: Pipe a handful of lines through `filter` against a stand-in service that answers
with known probabilities; the output is exactly the lines above the threshold, byte-identical and in
input order.

**Acceptance Scenarios**:

1. **Given** five lines on standard input and a question, **When** the user runs `jevpipe filter "<question>"`, **Then** each line the service answers with probability at or above 0.5 is printed unchanged, in input order, and no other output appears on standard output.
2. **Given** two files named as arguments, **When** the user runs `jevpipe filter "<question>" a.txt b.txt`, **Then** the lines of `a.txt` are judged and emitted before those of `b.txt`, each in its file's order.
3. **Given** `--threshold 0.8`, **When** a line's probability is 0.79, **Then** that line is not printed.
4. **Given** a run where at least one line is kept, **When** it finishes, **Then** the exit status is 0 and one summary line (records, kept, skipped, failed, cost, duration) is written to standard error.
5. **Given** a run where no line is kept and nothing failed, **When** it finishes, **Then** the exit status is 1.
6. **Given** the service fails transiently for some requests (rate limited, overloaded, timed out), **When** the run continues, **Then** those records are retried and, if a retry succeeds, the output is the same as without the failures.
7. **Given** a record still fails after all retries, **When** the run finishes, **Then** all other records are still judged and emitted, the failure is reported on standard error, it is counted in the summary, and the exit status is 2.
8. **Given** JSON lines (JSONL) on standard input, **When** the run completes, **Then** each line is judged as its text and kept lines are printed unchanged, so the output is still valid JSONL.
9. **Given** the output is piped into a consumer that stops reading early (such as `head -3`), **When** the consumer exits, **Then** jevpipe stops promptly, sends no further requests and prints no error.

---

### User Story 2 - Semantic grep over files (Priority: P2)

A developer or agent pipes a list of file paths (for example from `git ls-files`) into
`jevpipe filter "<question>" --read-files` and gets back the paths of the files whose content matches,
so the result can feed `xargs`, an editor or the agent's next step.

**Why this priority**: It is the flagship use case from the product idea ("find code by what it does")
and the most common way an agent will reach for jevpipe.

**Independent Test**: Create a few small text files plus one binary file, pipe their paths through
`filter --read-files` against the stand-in service; the output is the matching paths, the binary file
is skipped and counted.

**Acceptance Scenarios**:

1. **Given** a list of paths, **When** the user runs `jevpipe filter "<question>" --read-files`, **Then** each path's file content (together with its path) is judged, and the paths of matching files are printed unchanged, in input order.
2. **Given** a path to a binary file, an image or a file that is not valid UTF-8 text, **When** it is processed, **Then** it is skipped without a service call and counted as skipped in the summary; the exit status is not affected.
3. **Given** a path to an empty file or a directory, **When** it is processed, **Then** it is skipped and counted as skipped.
4. **Given** a path that does not exist or cannot be read, **When** it is processed, **Then** it is reported on standard error, counted as failed, and the exit status is 2.
5. **Given** a file larger than the service accepts, **When** it is processed, **Then** its content is cut to fit, the file is still judged, and the truncation is visible in the decision details (User Story 3).

---

### User Story 3 - Inspect the decisions (Priority: P3)

A script that needs the probabilities runs `filter` with `--json` and gets the kept records as JSON
objects carrying their probability. An agent tuning a question adds `--all` to also see the dropped,
skipped and failed records with their outcome.

**Why this priority**: Calibrated probabilities are the product's point of difference. The spike showed
that question phrasing strongly affects results, so agents need to see how well a question separates
records before trusting it.

**Independent Test**: Run `filter --json` and `filter --json --all` over three records (one kept, one
dropped, one skipped) against the stand-in service; the first prints one JSON line, the second three,
all valid for `jq`.

**Acceptance Scenarios**:

1. **Given** `--json`, **When** a run completes, **Then** standard output contains one JSON object line per kept record, the same records and order as without `--json`.
2. **Given** a kept record printed with `--json`, **Then** the object contains the record's position in the input (1-based), the original record and the probability, and marks whether the content was truncated.
3. **Given** `--json --all`, **When** a run completes, **Then** every input record produces exactly one JSON object line, in input order, with its outcome (kept, dropped, skipped or failed); dropped records carry their probability, skipped and failed records a short reason.
4. **Given** `--all` without `--json`, **When** the user runs it, **Then** it is rejected as a usage error, exit status 2.
5. **Given** `--json` with or without `--all`, **When** the run finishes, **Then** exit status and summary follow the same rules as without `--json`.

---

### Edge Cases

- **Undecided records** (skipped or failed) never appear in the default or `--json` output: output means "the service said yes", so a calling script never acts on a record nobody judged. They are visible on standard error, in the summary, through exit status 2 for failures, and with `--json --all`.
- **Blank lines** in the input are not records: they are neither judged nor emitted nor counted.
- **Empty input** (no records at all): no service call, summary reports zero records, exit status 1.
- **Missing or empty question**: usage error before any input is read, exit status 2.
- **A file named on the command line cannot be opened** (for example a typo): like `grep`, it is reported and the run continues with the other files. It counts as one failed record at its place in the input, with the file name as the record, so it shows up on standard error, in the summary, with `--json --all`, and makes the exit status 2. The same rule applies to a missing path in a `--read-files` list.
- **Threshold outside 0 to 1**: usage error, exit status 2.
- **No API key configured**: clear error naming the expected environment variable, before any input is read, exit status 2.
- **Run-level service errors** (key rejected, no credits left, unknown model, malformed request): the run stops at the first such error with a clear message and exit status 2, because every further record would fail the same way.
- **Service still rejects a truncated file as too large** (dense text such as minified code uses more tokens per character): the record fails with the reason "too large".
- **Rate limiting**: treated like any transient failure, retried after the wait the service asks for. A user who hits limits persistently lowers `--concurrency`.
- **A request hangs**: it is abandoned after a per-request timeout and treated as a transient failure.
- **Very long input stream**: memory use stays bounded; records are read, judged and emitted as a stream, not collected first.
- **Interrupt (Ctrl+C)**: the run stops promptly.
- **Standard input is a terminal and no files are given**: jevpipe reads standard input as a pipe tool does.
- **Windows and Unix line endings**: both are read correctly; kept records are written exactly as read.

## Requirements *(mandatory)*

### Functional Requirements

**Input**

- **FR-001**: `jevpipe filter <QUESTION> [FILE...]` MUST read records from the named files in argument order, or from standard input when no file is named or a file is named `-`.
- **FR-002**: By default each non-blank line MUST be one text record.
- **FR-003**: Lines that are JSON (JSONL) MUST be judged as text like any other line and emitted unchanged, so JSONL input yields JSONL output.
- **FR-004**: With `--read-files`, each record MUST be treated as a file path; the file's path and text content MUST be judged together, and the path is the record that is emitted.
- **FR-005**: With `--read-files`, binary, non-UTF-8, empty files and directories MUST be skipped without a service call; missing or unreadable paths MUST fail that record.
- **FR-006**: With `--read-files`, content beyond the service's size limit MUST be truncated to fit (about 100,000 characters, leaving room for the question); if the service still rejects it as too large, the record fails.

**Deciding**

- **FR-007**: Each record MUST be judged by one yes/no question sent as the question's instructions, without additional criteria.
- **FR-008**: A record MUST be kept when the answer's probability is at or above the threshold (`--threshold`, default 0.5, valid range 0 to 1).
- **FR-009**: Up to `--concurrency` requests (default 100) MUST be in flight at once, while output keeps the input order and is written as soon as each next record in order is decided.
- **FR-010**: Transient failures (rate limiting, overload, service errors, timeouts, connection failures) MUST be retried with increasing waits, honouring any wait the service asks for, a bounded number of times.
- **FR-011**: Each request MUST time out after a per-request limit (default 10 seconds, adjustable with `--request-timeout`) and count as a transient failure.
- **FR-012**: Run-level errors (authentication, missing credits, unknown model, malformed request) MUST stop the run immediately with a clear message.

**Output**

- **FR-013**: By default standard output MUST contain exactly the kept records, byte-identical to how they were read, one per line, in input order.
- **FR-014**: With `--json`, standard output MUST contain the same records as FR-013, each as one JSON object with the 1-based position, the original record and the probability; truncation MUST be marked.
- **FR-015**: With `--all` (valid only together with `--json`), standard output MUST contain one JSON object for every input record, in input order, with its outcome: kept or dropped with the probability, skipped or failed with a short reason.
- **FR-016**: Every run MUST end with exactly one summary line on standard error: records, kept, skipped, failed, total cost reported by the service, duration, and the model that answered. Per-record failures MUST each be reported on standard error before the summary.
- **FR-017**: Exit status MUST be 0 when at least one record was kept and none failed, 1 when none was kept and none failed, and 2 when any record failed or the run stopped on an error.
- **FR-018**: When the consumer of standard output goes away, jevpipe MUST stop sending requests and exit without printing an error.

**Configuration**

- **FR-019**: The API key MUST come from the `OPENROUTER_API_KEY` environment variable, never from a file or a command-line argument; any non-empty value MUST be accepted, since a proxy may inject the real key.
- **FR-020**: The service address MUST default to OpenRouter and be overridable through the `JEVPIPE_BASE_URL` environment variable, so tests and compatible servers can be used.
- **FR-021**: The model MUST default to the service's latest Jev (`~typesafe/jev-latest`) and be selectable with `--model`, for example a pinned version for reproducible runs.
- **FR-022**: `jevpipe --help` and `jevpipe filter --help` MUST describe every option; `jevpipe --version` MUST print the version.

**Verification**

- **FR-023**: All behaviour in this spec MUST be verifiable by automated tests that run the real binary against a local stand-in for the service, without network access or an API key. The stand-in's responses MUST follow the recorded real responses in [api-spike.md](api-spike.md).

### Key Entities

- **Record**: one unit of input: a text line, or a file path whose content is judged. Has a 1-based position in the whole input stream and its original bytes, which are what gets emitted.
- **Question**: the user's yes/no question, the same for every record in a run.
- **Decision**: the outcome for one record, one of:
  - **kept** or **dropped**: the service answered; the probability is at or above, or below, the threshold. A "no" is dropped, never failed.
  - **skipped**: the record is deliberately not judged (binary, non-UTF-8 or empty file, directory).
  - **failed**: the record could not be decided: unreadable path, a text line that is not valid UTF-8, service errors after all retries, or still too large after truncation.

  A decision also records whether the content was truncated.
- **Run summary**: counts of records, kept, skipped and failed, total cost and duration; determines the exit status.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In every test scenario, kept records are byte-identical to the input records and in input order, so `filter` output can replace a `grep` in any pipeline.
- **SC-002**: 1,000 short records are decided in under 30 seconds under normal service conditions (measured: about 100 decisions per second are achievable).
- **SC-003**: With a streaming input, the first kept record is written within 2 seconds of being read.
- **SC-004**: With 10% of service calls failing transiently (simulated), 100% of records are still decided and the output is identical to a run without failures.
- **SC-005**: In 100% of test scenarios, the exit status matches the rules of FR-017 and exactly one summary line is written.
- **SC-006**: The full automated test suite passes on Linux, Windows and macOS with no network access and no API key.
- **SC-007**: A user who knows `grep` can run a first semantic search over a repository from `jevpipe filter --help` alone, in under 5 minutes.

## Assumptions

- OpenRouter is the only provider in this milestone; its System One endpoint is used because it is TypeSafe's native format, so TypeSafe's own service (a later option) needs only a different base URL. A shared key name for several providers can follow later.
- Probabilities for identical requests can differ slightly between calls, and the default model moves with TypeSafe's releases; this milestone does not promise identical results across runs. The summary names the model that answered, and `--model` pins one when reproducibility matters.
- The size limit is enforced by characters, not tokens, because TypeSafe publishes neither Jev's tokenizer nor a token-counting endpoint (only the tokens used, after a request), and a generic tokenizer would be just as approximate; measured Rust code uses about 3.7 characters per token.
- The default concurrency of 100 is based on the spike (100 simultaneous calls without rate limiting); occasional rate limiting is absorbed by retries, persistent limits by a lower `--concurrency`.
- The per-record position is the record's identity for this milestone; using an `id` field from JSONL records belongs to `map`.
- JSONL needs no mode of its own: the spike sent the same record as a JSON object and as its text and got the same probabilities (0.82–0.83) and nearly the same token count; structured input returns if `map` needs fields such as `id`.

## Out of Scope

- `map` and `serve`.
- Caching identical requests, and budgets on records, cost or run time (the next milestone).
- Other providers than OpenRouter, and shared key names across providers.
- Inverted matching, sorting or ranking by probability, and review bands for uncertain answers (`--json` plus `jq` covers them).
- The agent skill (`skills/jevpipe/SKILL.md`) and the example pipelines; they follow once `filter` exists and use the phrasing lessons from the spike.

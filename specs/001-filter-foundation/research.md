# Research: Filter Foundation

Service behaviour (endpoint, latency, limits, error shapes) was measured live and is recorded in
[api-spike.md](api-spike.md). This file covers the implementation choices. Crate versions are the
latest stable releases on crates.io as of 2026-09-26.

## Async runtime and HTTP client

- **Decision**: `tokio` 1.53 (multi-thread runtime) with `reqwest` 0.13.5 (features `json`; defaults
  kept: rustls TLS, HTTP/2, system proxy).
- **Rationale**: 100 requests in flight is an I/O problem, and `reqwest` on `tokio` is the most used
  and best maintained HTTP stack in Rust. `reqwest` 0.13 defaults to rustls with
  `rustls-platform-verifier`, i.e. the operating system's certificate store, and honours
  `HTTPS_PROXY`. Both matter in Claude Code cloud sessions, where an egress proxy injects the API key.
  HTTP/2 multiplexes all requests over one connection to OpenRouter. Prototype: the full dependency
  set builds on Windows (MSVC) in about 40 s with no extra tools (`aws-lc-rs` ships what it needs).
- **Alternatives considered**: `ureq` (blocking) with a thread per request in flight is simpler but
  loses HTTP/2 multiplexing and cancellation on broken pipe; `hyper` directly is too low-level.

## Bounded, ordered concurrency

- **Decision**: `futures` 0.3.34 `StreamExt::buffered(concurrency)` over the record stream.
- **Rationale**: `buffered` keeps at most N futures running and yields results in input order, which
  is exactly FR-009, in one line. Dropping the stream cancels in-flight requests, which gives FR-018
  (stop on broken pipe) for free.
- **Trade-off accepted**: head-of-line blocking. A slow record (retrying) holds back output of later
  records, and the window does not advance until it finishes. With 0.3–0.5 s calls this is minor;
  `buffer_unordered` plus a reorder buffer would avoid it at the cost of extra code.
- **Alternatives considered**: `tokio::sync::Semaphore` with spawned tasks and a reorder map: more
  code for the same behaviour.

## Retries and timeouts

- **Decision**: `backon` 1.6.0 exponential backoff with jitter (0.5 s start, 8 s cap, 4 retries);
  `.when()` retries only transient errors; `.adjust()` replaces the delay with the service's
  `Retry-After` when present. Per-request timeout 10 s via `reqwest::ClientBuilder::timeout`: about 7× the slowest measured call (1.4 s with 100 in flight), short because a hung request holds back ordered output.
- **Rationale**: a proven crate instead of a hand-rolled loop, per project rules; `adjust` covers
  `Retry-After` without custom plumbing.
- **Alternatives considered**: `reqwest-retry` + `reqwest-middleware` 0.9/0.5: middleware layering
  and its own policy types for the same result; harder to classify errors by response body
  (`max_tokens_exceeded`).

## Error classification

| Response | Class | Effect |
|---|---|---|
| Connection error, timeout, `408`, `429`, `500`, `502`, `503`, `504`, `524`, `529` | transient | retried; record fails when retries run out |
| `400` with `max_tokens_exceeded`, `413` | record | record fails with "too large" |
| `400` otherwise, `401`, `402`, `403`, `404` | run | run stops (FR-012) |

- **Rationale**: from the spike, a `400` is a malformed request or unknown model (identical for every
  record) except the size case, which is recognisable in the message.

## Record reading

- **Decision**: read input on a plain `std::thread` with `BufRead::read_until(b'\n')`, keeping each
  line's raw bytes including its terminator, and hand records to the pipeline through a bounded
  `tokio::sync::mpsc` channel; a line that is empty or whitespace-only is not a record.
- **Why not `tokio::io::stdin`**: tokio reads stdin with a blocking read that cannot be cancelled, so
  the runtime's shutdown waits for the next input line (tokio docs). With `tail -f log | jevpipe … |
  head -1` jevpipe would never exit. A detached std thread does not hold up the process exit.
- **Rationale**: FR-013 requires kept records byte-identical to the input, so the original bytes
  (including `\r\n` on Windows files) are what gets written back; the state sent to the service is the
  line without its terminator. A text line that is not valid UTF-8 fails as a record (the service
  accepts text only).
- **No JSONL mode**: the same record sent as a JSON object and as its text got the same answers
  (0.82–0.83, 327 vs 323 tokens), and lines are emitted unchanged anyway, so JSONL input already
  yields JSONL output.

## File content (`--read-files`)

- **Decision**: read at most 400,000 bytes per file (the 100,000-character limit times the 4-byte
  UTF-8 maximum), so huge files are never fully loaded. Skip when the path is a directory, the file is
  empty, the first 8 KiB contain a NUL byte, or the content is not valid UTF-8 (tolerating a character
  cut by the read limit at the very end). Truncate to 100,000 characters on a character boundary. Send
  `{"path": ..., "content": ...}` as state.
- **Rationale**: the NUL-byte check is what `grep` and `git` use for binary detection; UTF-8
  validation covers images and other encodings. No crate needed: this is a few lines of `std`.
- **Alternatives considered**: `content_inspector` crate: an extra dependency for the same heuristic.

## Output and broken pipe

- **Decision**: write to a locked `std::io::stdout()` and flush after each record. A `BrokenPipe`
  write error ends the run quietly with exit status 0, like `ripgrep`; `print_stdout` is linted, so
  every write goes through this one writer. JSON output is `serde_json`.
- **Rationale**: flushing per record gives streaming output (SC-003). Exit 0 on broken pipe keeps
  `set -o pipefail` pipelines with `| head` working.

## Errors in code

- **Decision**: `thiserror` 2.0.21 for the typed errors that drive behaviour (transient, record, run
  errors); no `anyhow`.
- **Rationale**: every error ends in a known place (a record failure line or the run error message),
  so typed errors are enough and keep classification exhaustive.

## Test stand-in

- **Decision**: `wiremock` 0.6.5 in integration tests, with a custom `Respond` implementation that
  answers based on the request's state (for example, the probability is encoded in the test record),
  and fails on demand (every Nth first attempt, run-level errors, `max_tokens_exceeded`). Tests are
  `#[tokio::test(flavor = "multi_thread")]` and run the real binary with `assert_cmd`, pointing
  `JEVPIPE_BASE_URL` at the stand-in and setting a dummy `OPENROUTER_API_KEY`.
- **Rationale**: the real binary against an HTTP stand-in is what the project rules require;
  `wiremock` is the standard async HTTP mock, runs on localhost on all three CI platforms, and a
  custom responder keeps tests about behaviour rather than fixed request matching. Response bodies
  follow the recorded shapes in `api-spike.md`.
- **Alternatives considered**: `httpmock` 0.8.3: similar, smaller community.

## Licenses

- **Decision**: extend `deny.toml` `licenses.allow` with `ISC`, `BSD-3-Clause`, `MIT-0` and
  `CDLA-Permissive-2.0`.
- **Rationale**: prototype `cargo deny check licenses` on the dependency set: rustls, `aws-lc-rs`,
  `webpki-root-certs` and `subtle` use these permissive licenses; nothing copyleft.

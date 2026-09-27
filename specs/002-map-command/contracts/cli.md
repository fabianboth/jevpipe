# CLI Contract: `jevpipe filter` and `jevpipe map`

Replaces [../../001-filter-foundation/contracts/cli.md](../../001-filter-foundation/contracts/cli.md).

## Synopsis

```text
jevpipe filter [OPTIONS] <QUESTION> [FILES]...
jevpipe map [OPTIONS] <--questions <JSON>|--questions-file <FILE>> [FILES]...
```

Records come from each `FILE` in order, or from standard input when no `FILE` is given or a `FILE`
is `-`. Each non-blank line is one record, sent as text (JSONL lines too). A line over 100,000
characters fails as `too large` without a request.

## Options

| Option | Commands | Default | Meaning |
|---|---|---|---|
| `-q, --questions <JSON>` | `map` | — | the questions inline; exactly one of `-q` and `-f` is required |
| `-f, --questions-file <FILE>` | `map` | — | read the questions from this file |
| `--read-files` | both | off | each record is a file path; the file's path and content are judged |
| `--threshold <P>` | `filter` | `0.5` | keep a record when its probability is ≥ P; P in 0..=1 |
| `--concurrency <N>` | both | `100` | maximum requests in flight; N ≥ 1 |
| `--model <ID>` | both | `~typesafe/jev-latest` | model to ask |
| `--request-timeout <SECS>` | both | `10` | abandon a request after this many seconds and retry it; SECS ≥ 1 |

Removed: `filter --json` and `filter --all` (use `map`).

## Questions (`map`)

Inline with `-q` or in a file with `-f`: a JSON object of named questions in the System One format ([../contracts/service.md](service.md)),
sent unchanged:

```json
{
  "relevant": {"type": "noul", "instructions": "Is this failure worth a closer look?"},
  "kind": {"type": "choice", "instructions": "What kind of failure is this?",
           "criteria": {"flaky": "infra or timing", "real": "deterministic bug"}},
  "severity": {"type": "score", "instructions": "How severe is this failure?",
               "criteria": ["cosmetic", "annoying", "blocking"]}
}
```

Checked while the arguments are parsed, before any input is read; a failure is a usage error
(exit 2):

```text
error: invalid value 'questions.json' for '--questions-file <FILE>': question `kind`: a choice needs criteria with 1 to 255 options
```

## Environment

Unchanged: `OPENROUTER_API_KEY` (required, any non-empty value), `JEVPIPE_BASE_URL` (default
`https://openrouter.ai/api`; jevpipe appends `/v1/systemone`).

## Standard output

`filter`: each kept record's original bytes, including its line terminator; a final record without a
terminator gets `\n`.

`map`: one JSON object per record, in input order, flushed per line:

```json
{"record":"test_login_timeout: TimeoutError after 30s","answers":{"relevant":{"type":"noul","noul":0.82},"kind":{"type":"choice","choice":"flaky","probabilities":{"real":0,"flaky":1},"confidence":1},"severity":{"type":"score","score":1.04,"legend":{"0":"cosmetic","1":"annoying","2":"blocking"},"probabilities":{"0":0.07,"1":0.82,"2":0.11},"confidence":0.73}}}
{"record":"src/net.rs","answers":{...},"truncated":true}
{"record":"assets/logo.png","outcome":"skipped","reason":"binary"}
{"record":"src/gone.rs","outcome":"failed","reason":"not found"}
```

`record` is the input line as a string, without its terminator (the path with `--read-files`).
`answers` is the service's `answers` object exactly as returned. `truncated` appears only when true.

## Standard error

One line per failed record or input, then exactly one summary line:

```text
jevpipe: line 4: not found
jevpipe: missing.txt: not found
jevpipe: 4 records, 2 answered, 1 skipped, 1 failed, 1 truncated, $0.0021, 0.8s
```

`filter` says `kept` instead of `answered`. `truncated` is omitted when zero, the cost when the service
reports none. A run-level error replaces the summary with
`jevpipe: error: <message>`. Record content never appears on standard error.

## Exit status

| Status | `filter` | `map` |
|---|---|---|
| 0 | at least one record kept and none failed; or the output consumer went away | no record failed (empty input included); or the output consumer went away |
| 1 | no record kept and none failed (includes empty input) | not used |
| 2 | any record or input failed, a run-level service error, or a usage error | same |

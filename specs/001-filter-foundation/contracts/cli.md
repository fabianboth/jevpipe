# CLI Contract: `jevpipe filter`

## Synopsis

```text
jevpipe filter [OPTIONS] <QUESTION> [FILE]...
```

Records come from each `FILE` in order, or from standard input when no `FILE` is given or a `FILE`
is `-`.

## Options

| Option | Default | Meaning |
|---|---|---|
| `--read-files` | off | each record is a file path; the file's path and content are judged, the path is emitted |
| `--threshold <P>` | `0.5` | keep a record when its probability is ≥ P; P in 0..=1 |
| `--json` | off | write kept records as JSON objects instead of raw lines |
| `--all` | off | with `--json`: write every record with its outcome; usage error without `--json` |
| `--concurrency <N>` | `100` | maximum requests in flight; N ≥ 1 |
| `--model <ID>` | `~typesafe/jev-latest` | model to ask |
| `--request-timeout <SECS>` | `10` | abandon a request after this many seconds and retry it; SECS ≥ 1 |

## Environment

| Variable | Required | Meaning |
|---|---|---|
| `OPENROUTER_API_KEY` | yes | API key; any non-empty value is accepted (a proxy may replace it) |
| `JEVPIPE_BASE_URL` | no | service base URL; default `https://openrouter.ai/api` (jevpipe appends `/v1/systemone`) |

## Standard output

Default: each kept record's original bytes, including its line terminator; a final record without a
terminator gets `\n`.

`--json`: one object per kept record, one per line:

```json
{"position":3,"record":"src/input.rs","outcome":"kept","probability":0.94,"truncated":false}
```

`--json --all`: one object per record, in input order:

```json
{"position":1,"record":"src/main.rs","outcome":"dropped","probability":0.12,"truncated":false}
{"position":2,"record":"assets/logo.png","outcome":"skipped","reason":"binary"}
{"position":3,"record":"src/input.rs","outcome":"kept","probability":0.94,"truncated":false}
{"position":4,"record":"src/gone.rs","outcome":"failed","reason":"not found"}
```

`record` is the line as a string, without its terminator. `probability` and
`truncated` appear on kept and dropped records, `reason` on skipped and failed ones.

## Standard error

One line per failed record, then exactly one summary line:

```text
jevpipe: record 4 (src/gone.rs): not found
jevpipe: 4 records, 1 kept, 1 skipped, 1 failed, $0.0021, 0.8s, typesafe/jev-1.13-20260917
```

The cost is omitted when the service reports none, the model when no request was made. A run-level
error replaces the summary with `jevpipe: error: <message>`.

## Exit status

| Status | When |
|---|---|
| 0 | at least one record kept and none failed; or the output consumer went away |
| 1 | no record kept and none failed (includes empty input) |
| 2 | any record failed, a run-level service error, or a usage error |

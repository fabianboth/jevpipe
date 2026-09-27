# Service Contract: System One API (subset used by `filter` and `map`)

Extends [../../001-filter-foundation/contracts/service.md](../../001-filter-foundation/contracts/service.md):
endpoint, authentication, error body and error classes are unchanged. Question format checked against
the TypeSafe docs on 2026-09-27 (see [../research.md](../research.md)).

## Request

```json
{
  "model": "~typesafe/jev-latest",
  "state": "<string> | {\"path\": \"...\", \"content\": \"...\"}",
  "questions": { "<name>": { "type": "noul | choice | score", "instructions": "...", "criteria": "..." } }
}
```

One request per record. `questions` is `map`'s questions verbatim (from `-q` or `-f`) or
`{"match": {"type": "noul", "instructions": "<QUESTION>"}}` (`filter`).

## Response (`200`)

```json
{
  "model": "typesafe/jev-1.13-20260917",
  "answers": {
    "flaky": {"type": "noul", "noul": 0.82},
    "kind": {"type": "choice", "choice": "flaky", "probabilities": {"real": 0, "flaky": 1}, "confidence": 1},
    "severity": {"type": "score", "score": 1.04, "legend": {"0": "cosmetic", "1": "annoying", "2": "blocking"},
                 "probabilities": {"0": 0.07, "1": 0.82, "2": 0.11}, "confidence": 0.73}
  },
  "usage": {"input_tokens": 423, "output_tokens": 64, "cost": 0.000017766}
}
```

Fields jevpipe reads: `answers` (each answer's `type`, plus `noul` for `filter`), and
`usage.cost` (optional). Unknown fields are ignored; `map` passes `answers` through whole.

An answer missing for an asked question, of another type than asked, or a `noul` outside 0..=1
stops the run (exit 2), for example
``jevpipe: error: service error: unexpected answer to `kind`: a noul, asked for a choice``.

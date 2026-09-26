# Service Contract: System One API (subset used by `filter`)

Verified live; full examples in [../api-spike.md](../api-spike.md).

## Request

```http
POST {JEVPIPE_BASE_URL}/v1/systemone
Authorization: Bearer {OPENROUTER_API_KEY}
Content-Type: application/json
```

```json
{
  "model": "~typesafe/jev-latest",
  "state": "<string | object | array>",
  "questions": {
    "match": { "type": "noul", "instructions": "<QUESTION>" }
  }
}
```

One request per record, one `noul` question named `match`, no `criteria`.

## Response (`200`)

```json
{
  "model": "typesafe/jev-1.13-20260917",
  "answers": { "match": { "type": "noul", "noul": 0.9 } },
  "usage": { "input_tokens": 310, "output_tokens": 20, "cost": 0.00001302 },
  "id": "gen-dec-...",
  "provider": "TypeSafe"
}
```

Fields jevpipe reads: `model`, `answers.match.noul` (probability 0..=1), `usage.cost` (optional).
Unknown fields are ignored.

## Errors

Body: `{"error": {"message": "<text>", "code": <status>}}`. An optional `Retry-After` header (seconds)
sets the delay before a retry.

| Status | Class |
|---|---|
| connection error, timeout, `408`, `429`, `500`, `502`, `503`, `504`, `524`, `529` | transient: retry |
| `400` whose message contains `max_tokens_exceeded`, `413` | record: fails as "too large" |
| other `400`, `401`, `402`, `403`, `404` | run: stop with the message |

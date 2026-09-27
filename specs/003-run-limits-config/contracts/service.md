# Service Contract: OpenRouter System One (changes)

Extends [../../002-map-command/contracts/service.md](../../002-map-command/contracts/service.md):
request, response and answer checks are unchanged. Error handling checked against the OpenRouter
docs on 2026-09-27 (see [../research.md](../research.md) R1).

## Endpoint and key

`POST {base-url}/v1/systemone`, `base-url` from the config file (default `https://openrouter.ai/api`).
`Authorization: Bearer <key>` with the key from `OPENROUTER_API_KEY` or the keychain. Only OpenRouter's
System One API (or a server speaking it) is supported.

## Cost

`usage.cost` (US dollars, a JSON number) is read from every answer, rounded to nano-dollars and added
to the run's spend meter as the answer arrives. It stays optional: without `--max-cost` a missing
cost only leaves it out of the summary; with `--max-cost` it stops the run (exit 2).

## Error body

```json
{
  "error": {
    "code": 402,
    "message": "This request would exceed your available credits given your current in-flight requests. …",
    "metadata": {
      "reason": "in_flight_budget_exhausted",
      "limit_source": "openrouter_in_flight_budget",
      "remedy_hint": "Retry after your in-flight requests settle (see the Retry-After header). …"
    }
  }
}
```

jevpipe reads `error.message` (as before) and `error.metadata.limit_source` (new; optional).

## Error classes

| Response | Class | Effect |
|---|---|---|
| 408, 429, 500, 502, 503, 504, 524, 529, timeout, connection error | transient | retried with backoff, honouring `Retry-After` |
| 402 with `limit_source` `openrouter_in_flight_budget` | transient | retried, honouring `Retry-After` |
| 402 with `limit_source` `openrouter_key_limit` | key limit | the run stops (exit 3): the record is unprocessed, no new requests |
| 402 with `limit_source` `openrouter_credits` | credits | the run stops (exit 3) |
| 413; 400 containing `max_tokens_exceeded` | too large | the record fails |
| any other status, including 402 without `limit_source` | rejected | run-level error, exit 2 |

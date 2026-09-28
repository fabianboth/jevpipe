# Service Contract: two providers (changes)

Extends [../../003-run-limits-config/contracts/service.md](../../003-run-limits-config/contracts/service.md).
Request, answers and answer checks are unchanged. Checked live against both services on
2026-09-28 (see [../research.md](../research.md) R1–R3).

## Endpoint and key

`POST {base-url}/v1/systemone` with `Authorization: Bearer <key>`.

| Provider | Default `base-url` | Key |
|---|---|---|
| `openrouter` | `https://openrouter.ai/api` | `OPENROUTER_API_KEY` or keychain `jevpipe`/`openrouter-api-key` |
| `typesafe` | `https://api.typesafe.ai` | `TYPESAFE_API_KEY` or keychain `jevpipe`/`typesafe-api-key` |

The default `model` is `jev-latest`, accepted by both.

## Usage

```json
{"usage": {"input_tokens": 355, "output_tokens": 55, "cost": 0.00001491}}
{"usage": {"input_tokens": 355, "output_tokens": 55}}
```

The first reply is from OpenRouter and the second from TypeSafe.

jevpipe reads `input_tokens`, `output_tokens` (new) and `cost`, all optional to the parser. Tokens are
counted as their sum.

## Error body

jevpipe reads the message from either shape:

| Shape | Message taken |
|---|---|
| `{"error": {"message": "…", "metadata": {"limit_source": "…"}}}` (OpenRouter) | `error.message` |
| `{"detail": {"error_type": "…", "message": "…"}}` (TypeSafe) | `detail.message`, else `detail.error_type` |
| anything else (e.g. TypeSafe's 422 validation list) | none: the status alone, as today |

The run-level error reads `jevpipe: error: service error: 400 Bad Request: Unknown model: jev-1.13`.

## Error classes

Unchanged. TypeSafe's cases fall into them as follows:

| TypeSafe response | Class |
|---|---|
| 429, 529, 5xx, timeouts | transient (retried, `Retry-After` honoured) |
| 400 `max_tokens_exceeded` | too large: the record fails (existing body check) |
| 400 unknown model, 401, 403, 422 | rejected: run-level error with the message |
| an exhausted balance (undocumented) | rejected: run-level error with the message |

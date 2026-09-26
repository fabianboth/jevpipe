# API Spike: Jev via OpenRouter

**Date**: 2026-09-26
**Purpose**: Verify the wire format and service behaviour before specifying `filter`. All calls were made
against the live service; the responses below are verbatim (only an account id removed).

## Endpoint

`POST https://openrouter.ai/api/v1/systemone`, header `Authorization: Bearer <OPENROUTER_API_KEY>`,
`Content-Type: application/json`.

OpenRouter's alternative `POST /api/alpha/decisions` returned byte-identical responses for the same
request. System One is TypeSafe's native format (TypeSafe serves it at `api.typesafe.ai/v1/systemone`),
so a configurable base URL covers both providers.

Docs: [TypeSafe API](https://docs.typesafe.ai/api), [Noul](https://docs.typesafe.ai/primitives/noul.md),
[Models](https://docs.typesafe.ai/models.md),
[OpenRouter Decisions reference](https://openrouter.ai/docs/api/api-reference/alphadecisions/submit-a-decisions-questions-and-answers-request).

## Findings

| Topic | Result |
|---|---|
| `noul` without `criteria` | Works; `criteria` is optional per the docs |
| Models | `typesafe/jev-1.13` (pinned) and `~typesafe/jev-latest` both resolved to `typesafe/jev-1.13-20260917` |
| Latency | 0.3–0.5 s per call, small state |
| Concurrency | 20, 50 and 100 simultaneous calls: all `200`, median latency unchanged (~0.37 s), 100 calls in 1.6 s wall time; no `429` observed |
| Rate-limit headers | None in responses; limits are not observable in advance |
| Cost | `usage.cost` in every response; ~$0.000013 for a ~300-token call, ~$0.001 for a ~27k-token state |
| Size limit | Docs: 64k tokens per request, 32k for state plus the longest question. Rust source measured at ~3.7 characters per token: 120,000 characters (32,023 tokens) accepted, 140,000 rejected with `400 max_tokens_exceeded`. The service rejects; it does not truncate |
| Probabilities | Two decimals; repeated identical calls varied slightly (0.90 vs 0.89) |
| Errors | Always `{"error":{"message":…,"code":…}}` |

Quality note: asked "Does this code parse command line arguments?" about the first 20k–120k characters of
clap's own source, Jev answered 0.27–0.40. Question phrasing matters; the agent skill and examples must
be built from measured questions.

## Responses

### `noul` without criteria

Request:

```json
{"model":"typesafe/jev-1.13","state":"fn handle_input(buf: &mut InputBuffer, ev: KeyEvent) { buf.push(ev); if buf.len() > MAX { buf.pop_front(); } }","questions":{"match":{"type":"noul","instructions":"Does this code handle player input buffering?"}}}
```

`200`:

```json
{"model":"typesafe/jev-1.13-20260917","answers":{"match":{"type":"noul","noul":0.9}},"usage":{"input_tokens":310,"output_tokens":20,"cost":0.00001302},"id":"gen-dec-1790439314-rl9aKuqsLVjHr09c7Qvu","provider":"TypeSafe"}
```

### JSON state with `noul`, `choice` and `score`

Request:

```json
{"model":"typesafe/jev-1.13","state":{"test":"test_login_timeout","error":"TimeoutError: waited 30s for /api/session","attempts":3,"passed_on_retry":true},"questions":{"flaky":{"type":"noul","instructions":"Is this test failure caused by infrastructure or timing rather than a code bug?"},"kind":{"type":"choice","instructions":"What kind of failure is this?","criteria":{"flaky":"infra or timing","real":"deterministic bug"}},"severity":{"type":"score","instructions":"How severe is this failure?","criteria":["cosmetic","annoying","blocking"]}}}
```

`200`:

```json
{"model":"typesafe/jev-1.13-20260917","answers":{"flaky":{"type":"noul","noul":0.82},"kind":{"type":"choice","choice":"flaky","probabilities":{"real":0,"flaky":1},"confidence":1},"severity":{"type":"score","score":1.04,"legend":{"0":"cosmetic","1":"annoying","2":"blocking"},"probabilities":{"0":0.07,"1":0.82,"2":0.11},"confidence":0.73}},"usage":{"input_tokens":423,"output_tokens":64,"cost":0.000017766},"id":"gen-dec-1790439315-U5iToT0eB01osVp6vNXF","provider":"TypeSafe"}
```

### Errors

Unknown model, `400`:

```json
{"error":{"message":"Model typesafe/no-such-model does not exist","code":400}}
```

Unknown question type, `400`:

```json
{"error":{"message":"[\n  {\n    \"code\": \"invalid_union\",\n    \"errors\": [],\n    \"note\": \"No matching discriminator\",\n    \"discriminator\": \"type\",\n    \"options\": [\n      \"noul\",\n      \"choice\",\n      \"score\"\n    ],\n    \"path\": [\n      \"questions\",\n      \"q\",\n      \"type\"\n    ],\n    \"message\": \"Invalid discriminator value. Expected 'noul' | 'choice' | 'score'\"\n  }\n]","code":400}}
```

Missing `Authorization` header, `401`:

```json
{"error":{"message":"No cookie auth credentials found","code":401}}
```

State over the size limit, `400`:

```json
{"error":{"message":"HTTP 400: {\"detail\":{\"error_type\":\"max_tokens_exceeded\"}}","code":400}}
```

Documented but not observed: `402` insufficient credits, `429` rate limited, `5xx` and `524`/`529` for
provider errors, timeouts and overload.

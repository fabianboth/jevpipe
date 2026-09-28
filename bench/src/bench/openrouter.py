import os
from typing import TypedDict

from bench.limits import StageError

API = "https://openrouter.ai/api/v1"
_EXHAUSTED = frozenset({"openrouter_key_limit", "openrouter_credits"})


class _Metadata(TypedDict, total=False):
    limit_source: str


class _Error(TypedDict, total=False):
    message: str
    metadata: _Metadata


class ErrorBody(TypedDict, total=False):
    error: _Error


def authorization() -> dict[str, str]:
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        message = "OPENROUTER_API_KEY is not set"
        raise StageError(message)
    return {"Authorization": f"Bearer {key}"}


def exhausted(body: ErrorBody) -> bool:
    return body.get("error", {}).get("metadata", {}).get("limit_source") in _EXHAUSTED

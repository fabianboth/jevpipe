import asyncio
import math
import time
from collections import Counter
from collections.abc import Sequence
from dataclasses import dataclass
from typing import NotRequired, TypedDict, cast

import httpx

from bench import openrouter
from bench.limits import LimitReachedError

MODEL = "deepseek/deepseek-v4.1-flash"
SYSTEM = "You judge one file at a time. Answer with exactly one word: yes or no."
IN_FLIGHT = 100
_COMPLETIONS = f"{openrouter.API}/chat/completions"
_ATTEMPTS = 3
_TIMEOUT_SECONDS = 60
_EXHAUSTED_STATUS = 402
_PUNCTUATION = ".,!?:;\"'`*"
_WITHOUT_LOGPROBS = ("Novita",)


@dataclass(frozen=True)
class Item:
    question: str
    name: str
    code: str


@dataclass(frozen=True)
class Reply:
    answer: bool | None
    probability: float | None
    provider: str | None
    cost: float
    model: str | None


@dataclass(frozen=True)
class DeepSeekRun:
    answer: tuple[bool | None, ...]
    probability: tuple[float | None, ...]
    failed: int
    providers: dict[str, int]
    cost: float
    wall_seconds: float
    resolved_model: str


class TopLogprob(TypedDict):
    token: str
    logprob: float


class _TokenLogprob(TypedDict):
    top_logprobs: list[TopLogprob]


class _Logprobs(TypedDict):
    content: list[_TokenLogprob] | None


class _Message(TypedDict):
    content: str | None


class _Choice(TypedDict):
    message: _Message
    logprobs: NotRequired[_Logprobs | None]


class _Usage(TypedDict, total=False):
    cost: float


class Response(TypedDict, total=False):
    choices: list[_Choice]
    usage: _Usage
    provider: str
    model: str


@dataclass(frozen=True)
class _Session:
    client: httpx.AsyncClient
    slots: asyncio.Semaphore
    exhausted: asyncio.Event
    spent: list[float]


def request_body(item: Item) -> dict[str, object]:
    return {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": f"{item.question}\n\nFile: {item.name}\n\n{item.code}"},
        ],
        "temperature": 0,
        "max_tokens": 5,
        "reasoning": {"enabled": False},
        "logprobs": True,
        "top_logprobs": 5,
        "provider": {"require_parameters": True, "ignore": list(_WITHOUT_LOGPROBS)},
    }


def answer_of(content: str) -> bool | None:
    words = content.split()
    first = words[0].strip(_PUNCTUATION).lower() if words else ""
    match first:
        case "yes":
            return True
        case "no":
            return False
        case _:
            return None


def probability_of(top: Sequence[TopLogprob]) -> float | None:
    yes = _mass(top, "yes")
    no = _mass(top, "no")
    return yes / (yes + no) if yes + no > 0 else None


def _mass(top: Sequence[TopLogprob], word: str) -> float:
    return sum(
        math.exp(entry["logprob"]) for entry in top if entry["token"].strip().lower() == word
    )


def reply_of(response: Response) -> Reply | None:
    choices = response.get("choices")
    if not choices:
        return None
    choice = choices[0]
    logprobs = choice.get("logprobs")
    tokens = logprobs["content"] if logprobs else None
    return Reply(
        answer_of(choice["message"]["content"] or ""),
        probability_of(tokens[0]["top_logprobs"]) if tokens else None,
        response.get("provider"),
        response.get("usage", {}).get("cost", 0.0),
        response.get("model"),
    )


def run(items: Sequence[Item]) -> DeepSeekRun:
    return asyncio.run(_run(items))


async def _run(items: Sequence[Item]) -> DeepSeekRun:
    limits = httpx.Limits(max_connections=IN_FLIGHT, max_keepalive_connections=IN_FLIGHT)
    headers = openrouter.authorization()
    async with httpx.AsyncClient(
        headers=headers, timeout=_TIMEOUT_SECONDS, limits=limits
    ) as client:
        session = _Session(client, asyncio.Semaphore(IN_FLIGHT), asyncio.Event(), [])
        started = time.perf_counter()
        replies = await asyncio.gather(*(_ask(session, item) for item in items))
        wall_seconds = time.perf_counter() - started
    if session.exhausted.is_set():
        message = "the API key's spend limit or the account's credits are used up"
        raise LimitReachedError(message)
    return _collect(replies, sum(session.spent), wall_seconds)


def _collect(replies: Sequence[Reply | None], cost: float, wall_seconds: float) -> DeepSeekRun:
    answered = [reply for reply in replies if reply is not None]
    providers = Counter(reply.provider or "unknown" for reply in answered)
    models = Counter(reply.model for reply in answered if reply.model)
    return DeepSeekRun(
        tuple(reply.answer if reply else None for reply in replies),
        tuple(reply.probability if reply else None for reply in replies),
        len(replies) - len(answered),
        dict(providers.most_common()),
        cost,
        wall_seconds,
        models.most_common(1)[0][0] if models else MODEL,
    )


async def _ask(session: _Session, item: Item) -> Reply | None:
    kept: Reply | None = None
    async with session.slots:
        for attempt in range(_ATTEMPTS):
            if session.exhausted.is_set():
                return None
            reply = await _attempt(session, item)
            if reply is not None:
                session.spent.append(reply.cost)
                if reply.probability is not None:
                    return reply
                kept = reply
            await asyncio.sleep(2**attempt)
    return kept


async def _attempt(session: _Session, item: Item) -> Reply | None:
    try:
        response = await session.client.post(_COMPLETIONS, json=request_body(item))
    except httpx.HTTPError:
        return None
    if response.status_code == _EXHAUSTED_STATUS and openrouter.exhausted(
        cast("openrouter.ErrorBody", response.json())
    ):
        session.exhausted.set()
        return None
    if not response.is_success:
        return None
    try:
        return reply_of(cast("Response", response.json()))
    except ValueError:
        return None

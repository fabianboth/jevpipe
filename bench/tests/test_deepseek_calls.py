import json
import math
from collections.abc import Callable

import httpx
import pytest

from bench import deepseek, openrouter
from bench.limits import LimitReachedError

type Replies = Callable[[httpx.Request], httpx.Response]

_ITEM = deepseek.Item("Does this function sort a list?", "0000.py", "def f(x): return sorted(x)")


@pytest.fixture(autouse=True)
def waits(monkeypatch: pytest.MonkeyPatch) -> list[int]:
    waits: list[int] = []
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")

    def delay(attempt: int, _: httpx.Response | None) -> float:
        waits.append(attempt)
        return 0.0

    monkeypatch.setattr(openrouter, "delay", delay)
    return waits


def _answer(word: str, yes: float | None) -> httpx.Response:
    top = [] if yes is None else [{"token": "yes", "logprob": math.log(yes)}]
    top += [] if yes is None else [{"token": "no", "logprob": math.log(1 - yes)}]
    body = {
        "choices": [
            {"message": {"content": word}, "logprobs": {"content": [{"top_logprobs": top}]}}
        ],
        "usage": {"cost": 0.001},
        "provider": "Stand-in",
        "model": deepseek.MODEL,
    }
    return httpx.Response(200, json=body)


def _run(replies: list[httpx.Response]) -> tuple[deepseek.DeepSeekRun, int]:
    sent: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        sent.append(request)
        return replies[min(len(sent), len(replies)) - 1]

    return deepseek.run([_ITEM], httpx.MockTransport(handler)), len(sent)


def test_an_answer_with_a_probability_is_asked_once(waits: list[int]) -> None:
    found, sent = _run([_answer("yes", 0.8)])

    assert (found.answer, found.probability, found.failed) == ((True,), (pytest.approx(0.8),), 0)
    assert sent == 1
    assert waits == []


def test_transient_errors_are_retried_without_a_wait_after_the_last(waits: list[int]) -> None:
    found, sent = _run([httpx.Response(503)])

    assert found.failed == 1
    assert sent == openrouter.ATTEMPTS
    assert waits == list(range(openrouter.ATTEMPTS - 1))


def test_a_permanent_error_is_not_retried() -> None:
    found, sent = _run([httpx.Response(400, json={"error": {"message": "bad request"}})])

    assert (found.failed, sent) == (1, 1)


def test_an_answer_without_a_probability_is_asked_again_and_kept() -> None:
    found, sent = _run([_answer("no", None), _answer("no", None)])

    assert (found.answer, found.probability, found.failed) == ((False,), (None,), 0)
    assert sent == openrouter.ATTEMPTS
    assert found.cost == pytest.approx(0.001 * openrouter.ATTEMPTS)


def test_a_reply_with_neither_word_nor_probability_fails() -> None:
    found, _ = _run([_answer("maybe", None)])

    assert found.failed == 1


def test_a_malformed_reply_fails_the_record_not_the_run() -> None:
    found, sent = _run([httpx.Response(200, content=b"not json")])

    assert (found.failed, sent) == (1, 1)


def test_a_spent_key_stops_the_run() -> None:
    body = {"error": {"message": "limit", "metadata": {"limit_source": "openrouter_key_limit"}}}

    with pytest.raises(LimitReachedError):
        _run([httpx.Response(402, content=json.dumps(body).encode())])

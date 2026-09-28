import httpx

from bench import openrouter


def test_the_wait_the_service_asks_for_wins() -> None:
    response = httpx.Response(429, headers={"Retry-After": "7"})

    assert openrouter.delay(0, response) == 7.0


def test_waits_grow_with_jitter_up_to_a_cap() -> None:
    first = openrouter.delay(0, None)
    late = openrouter.delay(10, None)

    assert 0.5 <= first < 1.0
    assert 8.0 <= late < 16.0


def test_only_timeouts_rate_limits_and_server_errors_are_transient() -> None:
    assert all(openrouter.transient(status) for status in (408, 429, 500, 503, 529))
    assert not any(openrouter.transient(status) for status in (400, 401, 403, 404, 413))

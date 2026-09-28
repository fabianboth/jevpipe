import math

import pytest

from bench import deepseek


@pytest.mark.parametrize(
    ("content", "answer"),
    [
        ("yes", True),
        ("No.", False),
        ("no\n\nthis function sorts", False),
        ("maybe", None),
        ("", None),
    ],
)
def test_answer_is_the_first_word(content: str, answer: object) -> None:
    assert deepseek.answer_of(content) is answer


def _top(*entries: tuple[str, float]) -> list[deepseek.TopLogprob]:
    return [{"token": token, "logprob": math.log(p)} for token, p in entries]


def test_probability_is_yes_over_yes_and_no() -> None:
    assert deepseek.probability_of(_top(("yes", 0.6), ("no", 0.2), ("maybe", 0.2))) == (
        pytest.approx(0.75)
    )
    assert deepseek.probability_of(_top(("no", 0.9), ("yes", 0.1))) == pytest.approx(0.1)


def test_probability_adds_spaced_and_capitalised_tokens() -> None:
    top = _top((" Yes", 0.3), ("yes", 0.3), ("No", 0.2))

    assert deepseek.probability_of(top) == pytest.approx(0.75)


def test_no_probability_without_yes_or_no() -> None:
    assert deepseek.probability_of(_top(("maybe", 0.9))) is None


def test_a_response_without_choices_is_no_reply() -> None:
    assert deepseek.reply_of({"provider": "x"}) is None


def test_a_reply_carries_answer_probability_provider_and_cost() -> None:
    response: deepseek.Response = {
        "choices": [
            {
                "message": {"content": "Yes"},
                "logprobs": {"content": [{"top_logprobs": _top(("Yes", 0.8), ("No", 0.2))}]},
            }
        ],
        "usage": {"cost": 0.00005},
        "provider": "DeepInfra",
        "model": "deepseek/deepseek-v4.1-flash",
    }

    reply = deepseek.reply_of(response)

    assert reply is not None
    assert reply.answer is True
    assert reply.probability == pytest.approx(0.8)
    assert (reply.provider, reply.cost) == ("DeepInfra", 0.00005)

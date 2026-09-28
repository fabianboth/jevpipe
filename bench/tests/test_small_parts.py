import pytest

from bench import codex, licenses, wording
from bench.limits import LimitReachedError, StageError


def test_the_best_wording_wins_and_ties_go_to_the_first() -> None:
    assert wording.best_wording({1: 0.70, 2: 0.75, 3: 0.75}) == 2


def test_an_unknown_or_missing_license_reads_as_none() -> None:
    assert licenses.reported("MIT") == "MIT"
    assert licenses.reported("NOASSERTION") == "NOASSERTION"
    assert licenses.reported("") is None
    assert licenses.reported(None) is None


def test_a_usage_limit_is_read_from_the_end_of_codex_errors() -> None:
    limit = codex.failure(1, "starting\nERROR: You've hit your usage limit.")
    echoed = codex.failure(1, "prompt mentions a quota\n" + "working\n" * 10 + "ERROR: crashed")

    assert isinstance(limit, LimitReachedError)
    assert isinstance(echoed, StageError)


def test_a_failing_task_cancels_the_ones_still_waiting() -> None:
    ran: list[int] = []

    def fail() -> None:
        message = "boom"
        raise ValueError(message)

    def later(number: int) -> None:
        ran.append(number)

    tasks = [fail] + [lambda number=number: later(number) for number in range(50)]

    with pytest.raises(ValueError, match="boom"):
        codex.run_all(tasks, 1)
    assert len(ran) < 50

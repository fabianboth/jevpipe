import pytest

from bench import bootstrap


def test_a_clear_lead_is_above_zero_in_every_draw() -> None:
    units: list[bootstrap.Unit] = [((5, 0, 5), (1, 4, 5))] * 20

    gap = bootstrap.f1_gap(units)

    assert gap.observed == pytest.approx(0.8)
    assert gap.low == gap.high == gap.observed
    assert gap.above_zero == 1.0


def test_a_split_decision_spans_zero() -> None:
    units: list[bootstrap.Unit] = [((4, 0, 4), (0, 0, 4))] * 10 + [((0, 0, 4), (4, 0, 4))] * 10

    gap = bootstrap.f1_gap(units)

    assert gap.observed == 0.0
    assert gap.low < 0 < gap.high
    assert 0.3 < gap.above_zero < 0.7


def test_draws_are_reproducible() -> None:
    units: list[bootstrap.Unit] = [((3, 1, 4), (2, 2, 4)), ((1, 0, 2), (2, 1, 2))] * 5

    assert bootstrap.f1_gap(units) == bootstrap.f1_gap(units)

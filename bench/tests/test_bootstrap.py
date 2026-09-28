import pytest

from bench import bootstrap
from bench.counts import Counts


def _unit(first: tuple[int, int, int], second: tuple[int, int, int]) -> bootstrap.Unit:
    return Counts(*first), Counts(*second)


def test_a_clear_lead_is_above_zero_in_every_draw() -> None:
    units = [_unit((5, 0, 5), (1, 4, 5))] * 20

    gap = bootstrap.f1_gap([units])

    assert gap.observed == pytest.approx(0.8)
    assert gap.low == gap.high == gap.observed
    assert gap.above_zero == 1.0


def test_a_split_decision_spans_zero() -> None:
    units = [_unit((4, 0, 4), (0, 0, 4))] * 10 + [_unit((0, 0, 4), (4, 0, 4))] * 10

    gap = bootstrap.f1_gap([units])

    assert gap.observed == 0.0
    assert gap.low < 0 < gap.high
    assert 0.3 < gap.above_zero < 0.7


def test_draws_are_reproducible() -> None:
    units = [_unit((3, 1, 4), (2, 2, 4)), _unit((1, 0, 2), (2, 1, 2))] * 5

    assert bootstrap.f1_gap([units]) == bootstrap.f1_gap([units])


def test_each_language_keeps_its_share_of_searches() -> None:
    lead = [_unit((4, 0, 4), (0, 0, 4))] * 10
    behind = [_unit((0, 0, 4), (4, 0, 4))] * 10

    gap = bootstrap.f1_gap([lead, behind])

    assert gap.low == gap.high == gap.observed == 0.0

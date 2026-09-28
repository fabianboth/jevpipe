import random
from collections.abc import Sequence
from dataclasses import dataclass

from bench import counts, dataset
from bench.counts import Counts

DRAWS = 10_000
_LOW = 0.025
_HIGH = 0.975

type Unit = tuple[Counts, Counts]


@dataclass(frozen=True)
class Gap:
    observed: float
    low: float
    high: float
    above_zero: float


def f1_gap(strata: Sequence[Sequence[Unit]]) -> Gap:
    rng = random.Random(dataset.SEED)
    draws = sorted(
        _gap([unit for units in strata for unit in rng.choices(units, k=len(units))])
        for _ in range(DRAWS)
    )
    return Gap(
        _gap([unit for units in strata for unit in units]),
        draws[int(_LOW * DRAWS)],
        draws[int(_HIGH * DRAWS) - 1],
        sum(draw > 0 for draw in draws) / DRAWS,
    )


def _gap(units: Sequence[Unit]) -> float:
    first = counts.total(first for first, _ in units)
    second = counts.total(second for _, second in units)
    return first.f1 - second.f1

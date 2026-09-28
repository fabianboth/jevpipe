import random
from collections.abc import Sequence
from dataclasses import dataclass

from bench import dataset
from bench.counts import Counts

DRAWS = 10_000
_LOW = 0.025
_HIGH = 0.975

type Tally = tuple[int, int, int]
type Unit = tuple[Tally, Tally]


@dataclass(frozen=True)
class Gap:
    observed: float
    low: float
    high: float
    above_zero: float


def f1_gap(units: Sequence[Unit]) -> Gap:
    rng = random.Random(dataset.SEED)
    draws = sorted(_gap(rng.choices(units, k=len(units))) for _ in range(DRAWS))
    return Gap(
        _gap(units),
        draws[int(_LOW * DRAWS)],
        draws[int(_HIGH * DRAWS) - 1],
        sum(draw > 0 for draw in draws) / DRAWS,
    )


def _gap(units: Sequence[Unit]) -> float:
    return _f1([first for first, _ in units]) - _f1([second for _, second in units])


def _f1(tallies: Sequence[Tally]) -> float:
    found = sum(found for found, _, _ in tallies)
    false_hits = sum(false_hits for _, false_hits, _ in tallies)
    relevant = sum(relevant for _, _, relevant in tallies)
    return Counts(found, false_hits, relevant).f1

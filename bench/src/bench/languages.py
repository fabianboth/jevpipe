import functools
import operator
import statistics
from collections.abc import Sequence
from dataclasses import dataclass

from bench import bootstrap, counts, metrics
from bench.contenders import Contender, Model
from bench.counts import Counts

COMPARISONS: tuple[tuple[Contender, Contender], ...] = (
    ("jevpipe", "grep-agent"),
    ("jevpipe", "deepseek"),
    ("deepseek", "grep-agent"),
)


@dataclass(frozen=True)
class PerThousand:
    seconds: float
    cost: float


def pooled(everything: Sequence[metrics.Scored], contender: Contender) -> Counts:
    return counts.total(scored.results[contender].counts for scored in everything)


def sweep(everything: Sequence[metrics.Scored], model: Model) -> list[tuple[float, Counts]]:
    sweeps = [metrics.sweep(scored.data, model) for scored in everything]
    return [
        (points[0][0], counts.total(found for _, found in points))
        for points in zip(*sweeps, strict=True)
    ]


def bands(everything: Sequence[metrics.Scored], model: Model) -> list[metrics.Band]:
    per_language = [metrics.bands(scored.data, model) for scored in everything]
    return [functools.reduce(operator.add, group) for group in zip(*per_language, strict=True)]


def gap(everything: Sequence[metrics.Scored], pair: tuple[Contender, Contender]) -> bootstrap.Gap:
    first, second = pair
    strata = [
        list(zip(metrics.per_query(scored, first), metrics.per_query(scored, second), strict=True))
        for scored in everything
    ]
    return bootstrap.f1_gap(strata)


def searches(everything: Sequence[metrics.Scored]) -> int:
    return sum(len(scored.data.ids("test")) for scored in everything)


def decisions(everything: Sequence[metrics.Scored]) -> int:
    return sum(scored.data.pool_size * len(scored.data.ids("test")) for scored in everything)


def median_per_1000(everything: Sequence[metrics.Scored], contender: Contender) -> PerThousand:
    return PerThousand(
        statistics.median(seconds_per_1000(everything, contender)),
        statistics.median(
            scored.data.cost(q, contender) / scored.data.pool_size * 1000
            for scored in everything
            for q in scored.data.ids("test")
        ),
    )


def seconds_per_1000(
    everything: Sequence[metrics.Scored], contender: Contender
) -> tuple[float, ...]:
    return tuple(
        scored.data.seconds(q, contender) / scored.data.pool_size * 1000
        for scored in everything
        for q in scored.data.ids("test")
    )


def last_run(everything: Sequence[metrics.Scored]) -> str:
    return max(metrics.dates(scored.data)["last"] for scored in everything)


def names(everything: Sequence[metrics.Scored]) -> str:
    labels = [scored.data.suite.label for scored in everything]
    return labels[0] if len(labels) == 1 else ", ".join(labels[:-1]) + " and " + labels[-1]

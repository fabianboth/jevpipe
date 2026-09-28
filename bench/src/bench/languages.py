import statistics
from collections.abc import Sequence
from typing import Literal

from bench import bootstrap, metrics
from bench.counts import Counts

SHOWN: tuple[metrics.Contender, ...] = ("grep-agent", "jevpipe", "deepseek")
COMPARISONS: tuple[tuple[metrics.Contender, metrics.Contender], ...] = (
    ("jevpipe", "grep-agent"),
    ("jevpipe", "deepseek"),
    ("deepseek", "grep-agent"),
)


def pooled(everything: Sequence[metrics.Scored], contender: metrics.Contender) -> Counts:
    return _summed([scored.results[contender].counts for scored in everything])


def _summed(counts: Sequence[Counts]) -> Counts:
    return Counts(
        sum(count.found for count in counts),
        sum(count.false_hits for count in counts),
        sum(count.known_relevant for count in counts),
    )


def sweep(everything: Sequence[metrics.Scored], model: metrics.Model) -> list[tuple[float, Counts]]:
    sweeps = [metrics.sweep(scored.data, model) for scored in everything]
    return [
        (points[0][0], _summed([counts for _, counts in points]))
        for points in zip(*sweeps, strict=True)
    ]


def bands(everything: Sequence[metrics.Scored], model: metrics.Model) -> list[metrics.Band]:
    per_language = [metrics.bands(scored.data, model) for scored in everything]
    return [_merged(group) for group in zip(*per_language, strict=True)]


def _merged(group: Sequence[metrics.Band]) -> metrics.Band:
    decisions = sum(band.decisions for band in group)
    right = sum(band.decisions * band.accuracy for band in group)
    return metrics.Band(
        group[0].low, group[0].high, decisions, right / decisions if decisions else 0.0
    )


def gap(
    everything: Sequence[metrics.Scored], pair: tuple[metrics.Contender, metrics.Contender]
) -> bootstrap.Gap:
    first, second = pair
    units: list[bootstrap.Unit] = []
    for scored in everything:
        units += zip(
            metrics.per_query(scored.data, first, metrics.threshold_for(scored.thresholds, first)),
            metrics.per_query(
                scored.data, second, metrics.threshold_for(scored.thresholds, second)
            ),
            strict=True,
        )
    return bootstrap.f1_gap(units)


def searches(everything: Sequence[metrics.Scored]) -> int:
    return sum(len(scored.data.ids("test")) for scored in everything)


def decisions(everything: Sequence[metrics.Scored]) -> int:
    return sum(scored.data.pool_size * len(scored.data.ids("test")) for scored in everything)


def per_1000(everything: Sequence[metrics.Scored], model: metrics.Model) -> tuple[float, float]:
    return (
        statistics.median(seconds_per_1000(everything, model)),
        statistics.median(_per_search(everything, (model, "cost"))),
    )


def seconds_per_1000(
    everything: Sequence[metrics.Scored], model: metrics.Model
) -> tuple[float, ...]:
    return _per_search(everything, (model, "wall_seconds"))


def _per_search(
    everything: Sequence[metrics.Scored],
    measure: tuple[metrics.Model, Literal["wall_seconds", "cost"]],
) -> tuple[float, ...]:
    model, field = measure
    return tuple(
        scored.data.runs[query_id][model][field] / scored.data.pool_size * 1000
        for scored in everything
        for query_id in scored.data.ids("test")
    )


def last_run(everything: Sequence[metrics.Scored]) -> str:
    return max(metrics.dates(scored.data)["last"] for scored in everything)


def names(everything: Sequence[metrics.Scored]) -> str:
    labels = [scored.data.suite.label for scored in everything]
    return labels[0] if len(labels) == 1 else ", ".join(labels[:-1]) + " and " + labels[-1]

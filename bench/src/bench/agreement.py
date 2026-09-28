from collections.abc import Iterable, Mapping
from dataclasses import dataclass

from bench import counts, dataset, labels
from bench.counts import Counts

GATE_F1 = 0.67
MIN_RATED_TWICE = 30


@dataclass(frozen=True)
class Verdicts:
    counts: Counts
    pairs: int
    same: int

    @property
    def same_share(self) -> float | None:
        return self.same / self.pairs if self.pairs else None


@dataclass(frozen=True)
class Agreement:
    judge: Verdicts
    twice: int
    judge_on_twice: Verdicts | None
    experts: Verdicts | None

    @property
    def passed(self) -> bool:
        return self.judge.counts.f1 >= GATE_F1


def verdicts(pairs: Iterable[tuple[bool, bool]]) -> Verdicts:
    decided = list(pairs)
    return Verdicts(
        counts.count(decided), len(decided), sum(first == second for first, second in decided)
    )


def agreement(
    expert: Mapping[labels.Pair, tuple[int, ...]], judged: Mapping[labels.Pair, int]
) -> Agreement:
    rated = [pair for pair in judged if pair in expert]
    judge = verdicts((_relevant(judged[p]), _relevant(dataset.mean(expert[p]))) for p in rated)
    twice = [pair for pair in rated if len(expert[pair]) > 1]
    if len(twice) < MIN_RATED_TWICE:
        return Agreement(judge, len(twice), None, None)
    on_twice = verdicts(
        (_relevant(judged[p]), _relevant(dataset.mean(expert[p][1:]))) for p in twice
    )
    experts = verdicts(
        (_relevant(expert[p][0]), _relevant(dataset.mean(expert[p][1:]))) for p in twice
    )
    return Agreement(judge, len(twice), on_twice, experts)


def _relevant(rating: float) -> bool:
    return dataset.is_relevant(rating)


def describe(found: Agreement) -> str:
    judge = found.judge.counts
    between = (
        f"F1 {found.experts.counts.f1:.3f} on {found.twice} pairs rated twice"
        if found.experts is not None
        else f"only {found.twice} pairs rated twice"
    )
    return (
        f"judge: against the experts on {found.judge.pairs} rated pairs: "
        f"precision {judge.precision:.3f}, recall {judge.recall:.3f}, F1 {judge.f1:.3f}; "
        f"one expert against the others: {between} (gate {GATE_F1})"
    )

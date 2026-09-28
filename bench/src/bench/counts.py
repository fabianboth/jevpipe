from collections.abc import Iterable
from dataclasses import dataclass


@dataclass(frozen=True)
class Counts:
    found: float
    false_hits: float
    known_relevant: float

    @property
    def precision(self) -> float:
        flagged = self.found + self.false_hits
        return self.found / flagged if flagged else 0.0

    @property
    def recall(self) -> float:
        return self.found / self.known_relevant if self.known_relevant else 0.0

    @property
    def f1(self) -> float:
        total = self.precision + self.recall
        return 2 * self.precision * self.recall / total if total else 0.0

    def __add__(self, other: Counts) -> Counts:
        return Counts(
            self.found + other.found,
            self.false_hits + other.false_hits,
            self.known_relevant + other.known_relevant,
        )


NONE = Counts(0, 0, 0)


def total(counts: Iterable[Counts]) -> Counts:
    return sum(counts, NONE)


def count(decisions: Iterable[tuple[bool, bool]]) -> Counts:
    found = false_hits = known_relevant = 0
    for flagged, relevant in decisions:
        found += flagged and relevant
        false_hits += flagged and not relevant
        known_relevant += relevant
    return Counts(found, false_hits, known_relevant)

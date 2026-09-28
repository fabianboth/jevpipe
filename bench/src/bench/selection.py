import random
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, TypedDict, cast

from bench import dataset, decisions, labels, pool, runs, store
from bench.limits import StageError
from bench.records import RunFile

type Reason = Literal["validation", "hit", "grep-any-sample", "unflagged-sample"]

GREP_ANY_SAMPLE = 20
UNFLAGGED_SAMPLE = 10
BATCH = 40
_SELECTION_FILE = "selection.json"


@dataclass(frozen=True)
class Flags:
    jevpipe: frozenset[int]
    deepseek: frozenset[int]
    grep_any: frozenset[int]
    grep_all: frozenset[int]
    grep_agent: frozenset[int]

    def hits(self) -> frozenset[int]:
        return self.jevpipe | self.deepseek | self.grep_all | self.grep_agent

    def any_flag(self) -> frozenset[int]:
        return self.hits() | self.grep_any


@dataclass(frozen=True)
class Item:
    id: str
    query: str
    snippet: int
    reasons: tuple[Reason, ...]

    @property
    def validation(self) -> bool:
        return "validation" in self.reasons


@dataclass(frozen=True)
class Selection:
    items: list[Item]
    complete: bool

    def batches(self) -> list[list[Item]]:
        return batches(self.items)


class _ItemRecord(TypedDict):
    id: str
    query: str
    snippet: str
    reasons: list[Reason]


class _SelectionFile(TypedDict):
    seed: int
    complete: bool
    items: list[_ItemRecord]


class BatchFile(TypedDict):
    batch: int
    model: str
    judged: str
    seconds: float
    ratings: dict[str, int]


def flags_of(run: RunFile) -> Flags:
    lowest = decisions.LOWEST_THRESHOLD
    answered_yes = frozenset(i for i, a in enumerate(run["deepseek"]["answer"]) if a is True)
    return Flags(
        decisions.Setting("jevpipe", lowest).flags(run),
        decisions.Setting("deepseek", lowest).flags(run) | answered_yes,
        decisions.Setting("grep-any", lowest).flags(run),
        decisions.Setting("grep-all", lowest).flags(run),
        decisions.Setting("grep-agent", lowest).flags(run),
    )


def select(rated: Iterable[labels.Pair], flags: Mapping[str, Flags], pool_size: int) -> list[Item]:
    validation = validation_items(rated)
    rated_pairs = {(item.query, item.snippet) for item in validation}
    reasons: dict[labels.Pair, set[Reason]] = {}
    unrated = _Unrated(rated_pairs, pool_size)
    for query_id, flagged in sorted(flags.items()):
        for reason, chosen in unrated.reasons(query_id, flagged):
            for index in chosen:
                reasons.setdefault((query_id, index), set()).add(reason)
    others = sorted(reasons)
    random.Random(dataset.SEED).shuffle(others)
    return validation + [
        Item(f"j{number:05d}", query_id, index, tuple(sorted(reasons[query_id, index])))
        for number, (query_id, index) in enumerate(others, start=len(validation) + 1)
    ]


def validation_items(rated: Iterable[labels.Pair]) -> list[Item]:
    pairs = sorted(set(rated))
    random.Random(dataset.SEED).shuffle(pairs)
    return [
        Item(f"j{number:05d}", query_id, index, ("validation",))
        for number, (query_id, index) in enumerate(pairs, start=1)
    ]


@dataclass(frozen=True)
class _Unrated:
    rated: set[labels.Pair]
    pool_size: int

    def reasons(self, query_id: str, flagged: Flags) -> list[tuple[Reason, list[int]]]:
        any_hits = self._of(query_id, flagged.grep_any)
        unflagged = self._of(query_id, set(range(self.pool_size)) - flagged.any_flag())
        return [
            ("hit", self._of(query_id, flagged.hits())),
            ("grep-any-sample", _sample(any_hits, GREP_ANY_SAMPLE, f"{query_id}:grep-any")),
            ("unflagged-sample", _sample(unflagged, UNFLAGGED_SAMPLE, f"{query_id}:unflagged")),
        ]

    def _of(self, query_id: str, indexes: Iterable[int]) -> list[int]:
        return sorted(index for index in indexes if (query_id, index) not in self.rated)


def _sample(population: list[int], size: int, name: str) -> list[int]:
    rng = random.Random(f"{dataset.SEED}:{name}")
    return sorted(rng.sample(population, min(size, len(population))))


def batches(items: Sequence[Item]) -> list[list[Item]]:
    validation = [item for item in items if item.validation]
    others = [item for item in items if not item.validation]
    return _chunks(validation) + _chunks(others)


def _chunks(items: Sequence[Item]) -> list[list[Item]]:
    return [list(items[start : start + BATCH]) for start in range(0, len(items), BATCH)]


def current(suite: store.Suite) -> Selection:
    stored = _stored(suite)
    if stored is not None and stored.complete:
        return stored
    run_files = runs.load_runs(suite)
    complete = all(query.id in run_files for query in dataset.load(suite).queries)
    experts = labels.load_experts(suite)
    if complete:
        flags = {query_id: flags_of(run) for query_id, run in run_files.items()}
        chosen = select(experts, flags, len(pool.load(suite).snippets))
    else:
        chosen = validation_items(experts)
    if stored is not None and chosen[: len(stored.items)] != stored.items:
        message = "the stored selection differs from a fresh one; remove judge/ to start over"
        raise StageError(message)
    data: _SelectionFile = {
        "seed": dataset.SEED,
        "complete": complete,
        "items": [
            {
                "id": item.id,
                "query": item.query,
                "snippet": suite.name_of(item.snippet),
                "reasons": list(item.reasons),
            }
            for item in chosen
        ],
    }
    store.write_json(suite.judged / _SELECTION_FILE, data)
    return Selection(chosen, complete)


def _stored(suite: store.Suite) -> Selection | None:
    path = suite.judged / _SELECTION_FILE
    if not path.is_file():
        return None
    data = cast("_SelectionFile", store.read_json(path))
    items = [
        Item(item["id"], item["query"], pool.index_of_name(item["snippet"]), tuple(item["reasons"]))
        for item in data["items"]
    ]
    return Selection(items, data["complete"])


def selected(suite: store.Suite) -> list[Item]:
    stored = _stored(suite)
    return [] if stored is None else stored.items


def batch_file(suite: store.Suite, number: int) -> Path:
    return suite.judged / f"{number:03d}.json"


def judged_ratings(suite: store.Suite) -> dict[labels.Pair, int]:
    pairs = {item.id: (item.query, item.snippet) for item in selected(suite)}
    ratings: dict[labels.Pair, int] = {}
    for path in sorted(suite.judged.glob("[0-9][0-9][0-9].json")):
        data = cast("BatchFile", store.read_json(path))
        for item_id, rating in data["ratings"].items():
            ratings[pairs[item_id]] = rating
    return ratings


def finished(suite: store.Suite) -> bool:
    stored = _stored(suite)
    if stored is None or not stored.complete:
        return False
    return all(batch_file(suite, n).is_file() for n in range(len(stored.batches())))


def labels_of(suite: store.Suite) -> labels.Labels:
    return labels.Labels(labels.load_experts(suite), judged_ratings(suite))

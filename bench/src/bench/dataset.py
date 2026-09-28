import csv
import io
import random
from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass
from typing import Literal, TypedDict, cast

import httpx

from bench import store

REPOSITORY = "github/CodeSearchNet"
COMMIT = "106e827405c968597da938f6b373d30183918869"
FILE = "resources/annotationStore.csv"
SEED = 5
DEV_QUERIES = 20
REPEAT_QUERIES = 5
RELEVANT = 2.0

type Split = Literal["dev", "test"]
type Ratings = dict[tuple[str, str], tuple[int, ...]]

_SPLIT_FILE = "split.json"


@dataclass(frozen=True)
class Query:
    id: str
    text: str
    split: Split


@dataclass(frozen=True)
class Queries:
    queries: tuple[Query, ...]
    repeat: tuple[str, ...]

    def of_split(self, split: Split) -> tuple[Query, ...]:
        return tuple(query for query in self.queries if query.split == split)

    def by_id(self, query_id: str) -> Query:
        return next(query for query in self.queries if query.id == query_id)


class _QueryRecord(TypedDict):
    id: str
    text: str
    split: Split


class _SplitFile(TypedDict):
    seed: int
    queries: list[_QueryRecord]
    repeat: list[str]


def source() -> dict[str, str]:
    return {"repository": REPOSITORY, "commit": COMMIT, "file": FILE}


def download() -> str:
    cached = store.CACHE / "annotationStore.csv"
    if not cached.is_file():
        url = f"https://raw.githubusercontent.com/{REPOSITORY}/{COMMIT}/{FILE}"
        response = httpx.get(url, follow_redirects=True, timeout=60)
        response.raise_for_status()
        cached.parent.mkdir(parents=True, exist_ok=True)
        cached.write_bytes(response.content)
    return cached.read_text(encoding="utf-8")


def ratings_of(csv_text: str, suite: store.Suite) -> Ratings:
    ratings: defaultdict[tuple[str, str], list[int]] = defaultdict(list)
    for row in csv.DictReader(io.StringIO(csv_text)):
        if row["Language"] == suite.label:
            ratings[row["Query"], row["GitHubUrl"]].append(int(row["Relevance"]))
    return {pair: tuple(values) for pair, values in ratings.items()}


def mean(ratings: Iterable[int]) -> float:
    values = list(ratings)
    return sum(values) / len(values)


def is_relevant(rating: float) -> bool:
    return rating >= RELEVANT


def split(texts: Iterable[str]) -> Queries:
    ordered = sorted(set(texts))
    ids = [f"q{index:02d}" for index in range(len(ordered))]
    dev = set(random.Random(SEED).sample(ids, DEV_QUERIES))
    test = [query_id for query_id in ids if query_id not in dev]
    repeat = sorted(random.Random(SEED).sample(test, REPEAT_QUERIES))
    queries = tuple(
        Query(query_id, text, "dev" if query_id in dev else "test")
        for query_id, text in zip(ids, ordered, strict=True)
    )
    return Queries(queries, tuple(repeat))


def all_test(primary: Queries, texts: Iterable[str]) -> Queries:
    wanted = set(texts)
    return Queries(
        tuple(
            Query(query.id, query.text, "test") for query in primary.queries if query.text in wanted
        ),
        (),
    )


def save(queries: Queries, suite: store.Suite) -> None:
    data: _SplitFile = {
        "seed": SEED,
        "queries": [{"id": q.id, "text": q.text, "split": q.split} for q in queries.queries],
        "repeat": list(queries.repeat),
    }
    store.write_json(suite.results / _SPLIT_FILE, data)


def load(suite: store.Suite) -> Queries:
    data = cast("_SplitFile", store.read_json(suite.results / _SPLIT_FILE))
    queries = tuple(Query(q["id"], q["text"], q["split"]) for q in data["queries"])
    return Queries(queries, tuple(data["repeat"]))

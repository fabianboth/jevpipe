from collections import defaultdict
from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import cast

from bench import dataset, pool, store

type Pair = tuple[str, int]

_RATINGS_FILE = "ratings.json"


@dataclass(frozen=True)
class Labels:
    expert: Mapping[Pair, tuple[int, ...]]
    judge: Mapping[Pair, int] = field(default_factory=dict[Pair, int])

    def rating(self, pair: Pair) -> float | None:
        if pair in self.expert:
            return dataset.mean(self.expert[pair])
        return self.judge.get(pair)

    def relevant(self, pair: Pair) -> bool | None:
        rating = self.rating(pair)
        return None if rating is None else dataset.is_relevant(rating)

    def rated_by_expert(self, pair: Pair) -> bool:
        return pair in self.expert

    def with_judge(self, judge: Mapping[Pair, int]) -> Labels:
        return Labels(self.expert, judge)


def experts(
    queries: dataset.Queries, snippets: pool.Pool, ratings: dataset.Ratings
) -> dict[Pair, tuple[int, ...]]:
    query_ids = {query.text: query.id for query in queries.queries}
    indexes = snippets.index_of()
    return {
        (query_ids[text], indexes[url]): values
        for (text, url), values in ratings.items()
        if url in indexes
    }


def save_experts(expert: Mapping[Pair, tuple[int, ...]], suite: store.Suite) -> None:
    by_query: defaultdict[str, dict[str, list[int]]] = defaultdict(dict)
    for (query_id, index), values in sorted(expert.items()):
        by_query[query_id][suite.name_of(index)] = list(values)
    store.write_json(suite.results / _RATINGS_FILE, dict(sorted(by_query.items())))


def load_experts(suite: store.Suite) -> dict[Pair, tuple[int, ...]]:
    data = cast("dict[str, dict[str, list[int]]]", store.read_json(suite.results / _RATINGS_FILE))
    return {
        (query_id, pool.index_of_name(name)): tuple(values)
        for query_id, ratings in data.items()
        for name, values in ratings.items()
    }
